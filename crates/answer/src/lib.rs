//! The agent answer of surf: the typed blocks, one function that renders them to lines, one
//! quoting function for every value from a page, a server or an address, and the reverse of
//! both, [`unquote`] and [`parse`].
//!
//! The grammar is `docs/agent-answer.md`. Every line ends in a line feed and, but for the body
//! lines of the `text` block, is printable ASCII (U+0020 to U+007E). A value from outside is
//! written only through [`quote`] (or [`quote_cut`] for a field with a length limit), so it
//! stands inside double quotes and holds no `"`, no `\` and no character outside U+0020 to
//! U+007E. Every other part of a line is a number or a word of the format, and [`render`]
//! refuses a word outside its character set rather than write it.
//!
//! Page text in bulk is the `text` block ([`Text`]): every line of it behind `>` or `> `, so no
//! line of page text can stand where a line of the format stands. A body line holds printable
//! ASCII and UTF-8 characters but those [`escaped_in_text`] names, and does not start with one
//! [`escaped_at_run_start`] names; [`render`] refuses another. [`text_page`] cuts a rendering
//! into pages of at most [`TEXT_LINES_MAX`] lines and [`TEXT_BYTES_MAX`] bytes, and
//! [`quote_text`] writes a value of the tree outline.
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
    /// `now <shown> of <total>` and one indented line per actionable element shown.
    pub now: Option<Now>,
    /// `note` lines, in order.
    pub notes: Vec<Note>,
    /// The `text` block: a page's text, every line behind `>`, then the `more` line when lines
    /// are left. Last in the answer.
    pub text: Option<Text>,
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
            now: None,
            notes: Vec::new(),
            text: None,
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
    /// `unknown-handle`: the current document has no element of that handle, or there is no
    /// current document.
    UnknownHandle,
    /// `hidden`: the element or an ancestor has the `hidden` attribute.
    Hidden,
    /// `unresolved`: the link's target does not resolve against the document base URL.
    Unresolved,
    /// `address-too-long`: the link's resolved target is longer than the bound of an address
    /// `surf-fetch` follows, 65,536 bytes.
    AddressTooLong,
    /// `unsupported-scheme`: the link's resolved target has a scheme surf does not fetch.
    UnsupportedScheme,
    /// `forbidden-destination`: the page came from a public address and the link's target is
    /// a local one.
    ForbiddenDestination,
}

impl RefusalReason {
    const ALL: [RefusalReason; 12] = [
        RefusalReason::Token,
        RefusalReason::Closed,
        RefusalReason::Usage,
        RefusalReason::NoSuchSession,
        RefusalReason::SessionDied,
        RefusalReason::SessionTimeout,
        RefusalReason::UnknownHandle,
        RefusalReason::Hidden,
        RefusalReason::Unresolved,
        RefusalReason::AddressTooLong,
        RefusalReason::UnsupportedScheme,
        RefusalReason::ForbiddenDestination,
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
            RefusalReason::UnknownHandle => "unknown-handle",
            RefusalReason::Hidden => "hidden",
            RefusalReason::Unresolved => "unresolved",
            RefusalReason::AddressTooLong => "address-too-long",
            RefusalReason::UnsupportedScheme => "unsupported-scheme",
            RefusalReason::ForbiddenDestination => "forbidden-destination",
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

/// One indented line of the `path` block: `  <n> <command>`, then the handle of the element an
/// action took when it took one (`  3 follow l1`), then the argument quoted when it had one.
///
/// Without a handle the argument is an address, written whole. With a handle it is the
/// element's label, cut at [`LABEL_CHARS_MAX`] characters as the `now` block cuts it
/// (`  3 follow l1 "Sign in"`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathEntry {
    /// The action's sequence number in the session.
    pub sequence: u64,
    /// The command word, `[a-z]+`.
    pub command: String,
    /// The handle of the element the action took, a word of `[a-z0-9]+`; `None` for an action
    /// on no element.
    pub handle: Option<String>,
    /// The argument, quoted on the line; `None` for a command given none. With a handle, the
    /// kept part of the label when it was cut.
    pub argument: Option<String>,
    /// The label was cut at [`LABEL_CHARS_MAX`] characters: the field is followed by `...`. Only
    /// with a handle.
    pub argument_cut: bool,
}

/// The most characters of a label in the `now` block and on a `path` line (design section 5.2).
pub const LABEL_CHARS_MAX: usize = 60;

/// The most characters of a link's target in the `now` block. A deliberate bound the design does
/// not state: a target comes from the page, and an answer stays within its size.
pub const TARGET_CHARS_MAX: usize = 1_024;

/// The `now` block: the actionable elements of the current document, in document order, and
/// how many it has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Now {
    /// How many elements the document has; at least `elements.len()`.
    pub total: u64,
    /// The elements shown, the first ones in document order, one indented line each.
    pub elements: Vec<Element>,
}

/// One line of the `now` block: `  <handle> <kind> <visibility> "<label>" -> "<target>"`, with
/// ` unresolved` after a target that did not resolve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    /// The handle, a word of `[a-z0-9]+` (`l1`).
    pub handle: String,
    /// The kind, a word of `[a-z]+` (`link`).
    pub kind: String,
    /// `vis`, `hid` or `?`.
    pub visibility: Visibility,
    /// The label, cut at [`LABEL_CHARS_MAX`] characters.
    pub label: Cut,
    /// The target, cut at [`TARGET_CHARS_MAX`] characters.
    pub target: Target,
}

/// The visibility word of a `now` line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    /// `vis`: the element is visible.
    Visible,
    /// `hid`: the element is hidden.
    Hidden,
    /// `?`: visibility is not computed.
    Unknown,
}

impl Visibility {
    const ALL: [Visibility; 3] = [Visibility::Visible, Visibility::Hidden, Visibility::Unknown];

    /// The word of the format.
    pub fn word(self) -> &'static str {
        match self {
            Visibility::Visible => "vis",
            Visibility::Hidden => "hid",
            Visibility::Unknown => "?",
        }
    }
}

/// A link's target on a `now` line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// `-> "<target>"`: resolved against the document base URL, written as its path, query and
    /// fragment when it has the page's origin and whole otherwise.
    Resolved(Cut),
    /// `-> "<target>" unresolved`: the value as the page wrote it, which does not resolve.
    Unresolved(Cut),
}

/// A `note` line, from a fixed list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `note parser: <parts>`: the parts of the tree builder not implemented yet, at least one,
    /// each a word of `[a-z-]+`, joined by `,`.
    Parser(Vec<String>),
    /// `note visibility: hidden attribute only`: the `now` block says `hid` for an element the
    /// `hidden` attribute hides, and `?` for every other, as visibility is not computed.
    Visibility,
    /// `note text: <what>`: what the `text` block cannot show.
    Text(TextNote),
}

/// The words of `note visibility:`.
const VISIBILITY_NOTE: &str = "note visibility: hidden attribute only";

/// What a `note text:` line says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextNote {
    /// `no-css`: the text is computed from the tree without CSS, so text a style sheet hides is
    /// shown.
    NoCss,
    /// `no-document`: the load failed or the page is not HTML, so there is no text block.
    NoDocument,
    /// `deep <n>`: the Markdown wrote up to `n` levels of nested lists and quotes, at least 1,
    /// at the indentation of the level above them, without their markers, because the prefix of
    /// their lines would pass its bound (`surf_page_text::markdown::PREFIX_BYTES_MAX`).
    Deep(u64),
}

impl TextNote {
    const WORDS: [TextNote; 2] = [TextNote::NoCss, TextNote::NoDocument];

    /// The word of the format.
    pub fn word(self) -> &'static str {
        match self {
            TextNote::NoCss => "no-css",
            TextNote::NoDocument => "no-document",
            TextNote::Deep(_) => "deep",
        }
    }
}

/// The most body lines one `text` block shows.
pub const TEXT_LINES_MAX: usize = 200;

/// The most bytes the body lines of one `text` block hold, each counted with its `>` or `> `
/// and its line feed.
pub const TEXT_BYTES_MAX: u64 = 32_768;

/// What `[cut]` adds to a line cut on the block's byte bound.
const CUT_MARK: &str = "[cut]";

/// The form of the page text in a `text` block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// `tree`: the outline of the document's tree.
    Tree,
    /// `markdown`: the content as Markdown.
    Markdown,
}

impl Form {
    const ALL: [Form; 2] = [Form::Tree, Form::Markdown];

    /// The word of the format.
    pub fn word(self) -> &'static str {
        match self {
            Form::Tree => "tree",
            Form::Markdown => "markdown",
        }
    }
}

/// The `text` block: one page of the lines of a rendering.
///
/// The header is `text <form> <shown> of <total> lines from <first>, <bytes shown> of <bytes
/// total> bytes`; `<shown>` is `lines.len()` and `<bytes shown>` is counted from `lines`. Each
/// line is written `>` when it is empty and `> ` and the line otherwise. When `first + shown`
/// is at most `total`, the block ends in `more <n> lines: look --as <form> --from-line <m>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text {
    /// The form of the lines.
    pub form: Form,
    /// The number of the first line shown, counted from 1 over the whole rendering.
    pub first: u64,
    /// How many lines the whole rendering has.
    pub total: u64,
    /// How many bytes the whole rendering has as body lines (prefix and line feed counted).
    pub bytes_total: u64,
    /// The lines shown, without their prefix.
    pub lines: Vec<String>,
}

// --- Characters of page text ----------------------------------------------------------------

/// Whether a character of page text is written as `\x` and two lower-case hexadecimal digits
/// per UTF-8 byte in a `text` block, wherever it stands.
///
/// The ranges are rule 2 of section 3 of `docs/design/page-as-text.md` (decision D1, B), the one
/// place of the rule: C0, DEL and C1 (U+0000 to U+001F, U+007F to U+009F); the invisible,
/// format, bidirectional and separator characters and variation selectors (U+00AD, U+034F,
/// U+061C, U+115F to U+1160, U+17B4 to U+17B5, U+180B to U+180F, U+200B to U+200F, U+2028 to
/// U+202E, U+2060 to U+206F, U+3164, U+FE00 to U+FE0F, U+FEFF, U+FFA0, U+FFF0 to U+FFFB, U+1BCA0
/// to U+1BCA3, U+1D173 to U+1D17A); private use (U+E000 to U+F8FF, U+F0000 to U+10FFFF);
/// U+40000 to U+EFFFF; the noncharacters U+FDD0 to U+FDEF and U+nFFFE, U+nFFFF of planes 0 to
/// 3; and the ten fullwidth forms of surf's syntax (U+FF02, U+FF08, U+FF09, U+FF1C, U+FF1E,
/// U+FF20, U+FF3B, U+FF3C, U+FF3D, U+FF5C). U+FFFD is kept. Unassigned code points of planes 0
/// to 3 are not detected.
pub fn escaped_in_text(c: char) -> bool {
    let u = c as u32;
    matches!(
        u,
        0x0000..=0x001F
            | 0x007F..=0x009F
            | 0x00AD
            | 0x034F
            | 0x061C
            | 0x115F..=0x1160
            | 0x17B4..=0x17B5
            | 0x180B..=0x180F
            | 0x200B..=0x200F
            | 0x2028..=0x202E
            | 0x2060..=0x206F
            | 0x3164
            | 0xFE00..=0xFE0F
            | 0xFEFF
            | 0xFFA0
            | 0xFFF0..=0xFFFB
            | 0x1BCA0..=0x1BCA3
            | 0x1D173..=0x1D17A
            | 0xE000..=0xF8FF
            | 0xF0000..=0x10FFFF
            | 0x40000..=0xEFFFF
            | 0xFDD0..=0xFDEF
            | 0xFF02
            | 0xFF08
            | 0xFF09
            | 0xFF1C
            | 0xFF1E
            | 0xFF20
            | 0xFF3B
            | 0xFF3C
            | 0xFF3D
            | 0xFF5C
    ) || (u & 0xFFFE == 0xFFFE && u >> 16 <= 3)
}

/// Whether a character is escaped when it starts a run of page text (the first code point after
/// a character surf wrote): the combining marks U+0300 to U+036F, U+1AB0 to U+1AFF, U+1DC0 to
/// U+1DFF, U+20D0 to U+20FF and U+FE20 to U+FE2F (rule 2 of `docs/design/page-as-text.md`
/// section 3). Inside a run they are kept.
pub fn escaped_at_run_start(c: char) -> bool {
    matches!(
        c as u32,
        0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F
    )
}

/// Write a character as `\x` and two lower-case hexadecimal digits per UTF-8 byte.
fn push_escaped(character: char, out: &mut String) {
    let mut bytes = [0u8; 4];
    for byte in character.encode_utf8(&mut bytes).bytes() {
        out.push_str(&format!("\\x{byte:02x}"));
    }
}

/// Write one value from a page as a quoted field of a `text` block (the tree's texts and
/// attribute values).
///
/// As [`quote`], but a character above U+007F is written as itself unless [`escaped_in_text`]
/// names it, or it is the first character of the value and [`escaped_at_run_start`] names it.
/// `"` and `\` are escaped, so the field ends at its closing quote.
pub fn quote_text(value: &str) -> String {
    let mut field = String::with_capacity(value.len() + 2);
    field.push('"');
    for (index, character) in value.chars().enumerate() {
        if character == '"'
            || character == '\\'
            || escaped_in_text(character)
            || (index == 0 && escaped_at_run_start(character))
        {
            push_escaped(character, &mut field);
        } else {
            field.push(character);
        }
    }
    field.push('"');
    field
}

/// [`quote_text`] with a length limit of `limit` characters, cut as [`quote_cut`] cuts: a longer
/// value keeps its first `limit` characters and the field is followed by `...`.
pub fn quote_text_cut(value: &str, limit: usize) -> String {
    match value.char_indices().nth(limit) {
        Some((end, _)) => {
            let mut field = quote_text(&value[..end]);
            field.push_str("...");
            field
        }
        None => quote_text(value),
    }
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

impl Cut {
    /// `value` cut as [`quote_cut`] cuts it at `limit` characters: its first `limit` characters
    /// and `cut` when it is longer, the whole value otherwise.
    pub fn of(value: &str, limit: usize) -> Cut {
        match value.char_indices().nth(limit) {
            Some((end, _)) => Cut {
                kept: value[..end].to_string(),
                cut: true,
            },
            None => Cut {
                kept: value.to_string(),
                cut: false,
            },
        }
    }

    /// Whether [`unquote_cut`] with `limit` reads the field of this value back to it: a cut
    /// value keeps exactly `limit` characters, an uncut one at most `limit`.
    fn fits(&self, limit: usize) -> bool {
        let length = self.kept.chars().count();
        if self.cut {
            length == limit
        } else {
            length <= limit
        }
    }
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
    /// The `now` block shows more elements than its total.
    NowLongerThanTotal,
    /// A field with a length limit that its reading could not give back: a value marked cut
    /// that does not keep exactly its limit, an uncut one over it, or a `path` argument marked
    /// cut on a line without a handle or without an argument.
    CutLength {
        /// Which field, such as `now label`.
        field: &'static str,
    },
    /// A `note parser` line with no part, or a `note text: deep` line of 0 levels.
    EmptyNote,
    /// A body line of the `text` block holds a line feed or a character [`escaped_in_text`]
    /// names, or starts with one [`escaped_at_run_start`] names.
    TextLine {
        /// The body line, counted from 1 in the block.
        line: u64,
    },
    /// The `text` block shows more than [`TEXT_LINES_MAX`] lines or more than
    /// [`TEXT_BYTES_MAX`] bytes.
    TextOverBound,
    /// The counts of the `text` block contradict its lines: more lines shown than the total,
    /// more bytes shown than the total, a first line of 0, or lines shown past the total.
    TextCounts,
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
            RenderError::NowLongerThanTotal => {
                f.write_str("the now block shows more elements than its total")
            }
            RenderError::CutLength { field } => {
                write!(f, "the {field} does not fit its length limit as marked")
            }
            RenderError::EmptyNote => {
                f.write_str("a note parser line has no part, or a note text deep line 0 levels")
            }
            RenderError::TextLine { line } => {
                write!(
                    f,
                    "body line {line} of the text block holds a character it must escape"
                )
            }
            RenderError::TextOverBound => write!(
                f,
                "the text block shows more than {TEXT_LINES_MAX} lines or {TEXT_BYTES_MAX} bytes"
            ),
            RenderError::TextCounts => f.write_str("the counts of the text block contradict it"),
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
    if let Some(now) = &answer.now {
        render_now(now, &mut out)?;
    }
    for note in &answer.notes {
        render_note(note, &mut out)?;
    }
    if let Some(text) = &answer.text {
        render_text(text, &mut out)?;
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
        match &entry.handle {
            Some(handle) => {
                out.push(' ');
                out.push_str(checked(handle, "path handle", is_handle_char)?);
                match &entry.argument {
                    Some(argument) => {
                        let label = Cut {
                            kept: argument.clone(),
                            cut: entry.argument_cut,
                        };
                        out.push(' ');
                        push_cut(&label, LABEL_CHARS_MAX, "path label", out)?;
                    }
                    // A cut mark goes after a label: without one it cannot be written.
                    None if entry.argument_cut => {
                        return Err(RenderError::CutLength {
                            field: "path label",
                        });
                    }
                    None => {}
                }
            }
            None => {
                if entry.argument_cut {
                    return Err(RenderError::CutLength {
                        field: "path argument",
                    });
                }
                push_argument(entry.argument.as_deref(), out);
            }
        }
        out.push('\n');
    }
    Ok(())
}

fn render_now(now: &Now, out: &mut String) -> Result<(), RenderError> {
    let shown = now.elements.len() as u64;
    if shown > now.total {
        return Err(RenderError::NowLongerThanTotal);
    }
    out.push_str(&format!("now {shown} of {}\n", now.total));
    for element in &now.elements {
        let handle = checked(&element.handle, "now handle", is_handle_char)?;
        let kind = checked(&element.kind, "now kind", is_command_char)?;
        out.push_str(&format!("  {handle} {kind} {} ", element.visibility.word()));
        push_cut(&element.label, LABEL_CHARS_MAX, "now label", out)?;
        out.push_str(" -> ");
        match &element.target {
            Target::Resolved(target) => {
                push_cut(target, TARGET_CHARS_MAX, "now target", out)?;
            }
            Target::Unresolved(target) => {
                push_cut(target, TARGET_CHARS_MAX, "now target", out)?;
                out.push_str(" unresolved");
            }
        }
        out.push('\n');
    }
    Ok(())
}

/// The field of a value with a length limit, as [`quote_cut`] writes it, when [`unquote_cut`]
/// reads it back to the same value.
fn push_cut(
    value: &Cut,
    limit: usize,
    field: &'static str,
    out: &mut String,
) -> Result<(), RenderError> {
    if !value.fits(limit) {
        return Err(RenderError::CutLength { field });
    }
    out.push_str(&quote(&value.kept));
    if value.cut {
        out.push_str("...");
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
        Note::Visibility => out.push_str(VISIBILITY_NOTE),
        Note::Text(what) => {
            out.push_str("note text: ");
            out.push_str(what.word());
            if let TextNote::Deep(levels) = what {
                if *levels == 0 {
                    return Err(RenderError::EmptyNote);
                }
                out.push_str(&format!(" {levels}"));
            }
        }
    }
    out.push('\n');
    Ok(())
}

/// The bytes a line takes as a body line: `>` and a line feed when it is empty, `> `, the line
/// and a line feed otherwise.
fn body_bytes(line: &str) -> u64 {
    if line.is_empty() {
        2
    } else {
        line.len() as u64 + 3
    }
}

/// Whether a line may stand behind `> `: no character [`escaped_in_text`] names (a line feed
/// among them), and no character [`escaped_at_run_start`] names as its first.
fn is_body_line(line: &str) -> bool {
    !line.chars().next().is_some_and(escaped_at_run_start) && !line.chars().any(escaped_in_text)
}

fn push_body_line(line: &str, out: &mut String) {
    if line.is_empty() {
        out.push_str(">\n");
    } else {
        out.push_str("> ");
        out.push_str(line);
        out.push('\n');
    }
}

fn text_header(
    form: Form,
    shown: u64,
    total: u64,
    first: u64,
    bytes: u64,
    bytes_total: u64,
) -> String {
    format!(
        "text {} {shown} of {total} lines from {first}, {bytes} of {bytes_total} bytes",
        form.word()
    )
}

/// The `more` line of a page whose lines from `first + shown` on are left, or `None` when none
/// is left.
fn more_line(form: Form, first: u64, shown: u64, total: u64) -> Option<String> {
    let next = first.checked_add(shown)?;
    (next <= total).then(|| {
        format!(
            "more {} lines: look --as {} --from-line {next}",
            total - next + 1,
            form.word()
        )
    })
}

/// Check a `text` block and return the bytes of its body lines.
fn check_text(text: &Text) -> Result<u64, RenderError> {
    for (index, line) in text.lines.iter().enumerate() {
        if !is_body_line(line) {
            return Err(RenderError::TextLine {
                line: index as u64 + 1,
            });
        }
    }
    let shown = text.lines.len() as u64;
    let bytes: u64 = text.lines.iter().map(|line| body_bytes(line)).sum();
    if text.lines.len() > TEXT_LINES_MAX || bytes > TEXT_BYTES_MAX {
        return Err(RenderError::TextOverBound);
    }
    counts_hold(shown, text.total, text.first, bytes, text.bytes_total)
        .then_some(bytes)
        .ok_or(RenderError::TextCounts)
}

/// Whether the counts of a header agree with each other.
fn counts_hold(shown: u64, total: u64, first: u64, bytes: u64, bytes_total: u64) -> bool {
    let past_total = shown > 0 && first.checked_add(shown - 1).is_none_or(|last| last > total);
    shown <= total && bytes <= bytes_total && first >= 1 && !past_total
}

fn render_text(text: &Text, out: &mut String) -> Result<(), RenderError> {
    let bytes = check_text(text)?;
    let shown = text.lines.len() as u64;
    out.push_str(&text_header(
        text.form,
        shown,
        text.total,
        text.first,
        bytes,
        text.bytes_total,
    ));
    out.push('\n');
    for line in &text.lines {
        push_body_line(line, out);
    }
    if let Some(more) = more_line(text.form, text.first, shown, text.total) {
        out.push_str(&more);
        out.push('\n');
    }
    Ok(())
}

/// One page of a rendering's lines, from line `from` (counted from 1; 0 reads as 1).
///
/// Reads the lines one at a time and keeps those from `from` on while the page holds at most
/// [`TEXT_LINES_MAX`] lines and [`TEXT_BYTES_MAX`] bytes; every line is counted, with its bytes
/// as a body line, so the caller keeps only the lines shown. A line that alone exceeds the byte
/// bound is shown alone when it is the page's first, cut at the last escape unit that leaves
/// room for `[cut]`: a unit is one character, a `\` with the character after it, or the `\x`
/// escapes of one UTF-8 character, so the cut never leaves a bare `\` or part of a character.
pub fn text_page(form: Form, lines: impl Iterator<Item = String>, from: u64) -> Text {
    let first = from.max(1);
    let mut kept = Vec::new();
    let mut kept_bytes = 0;
    let mut keeping = true;
    let mut total = 0;
    let mut bytes_total = 0;
    for line in lines {
        total += 1;
        let bytes = body_bytes(&line);
        bytes_total += bytes;
        if !keeping || total < first {
            continue;
        }
        if kept.len() < TEXT_LINES_MAX && kept_bytes + bytes <= TEXT_BYTES_MAX {
            kept_bytes += bytes;
            kept.push(line);
        } else {
            if kept.is_empty() {
                kept.push(cut_line(&line));
            }
            keeping = false;
        }
    }
    Text {
        form,
        first,
        total,
        bytes_total,
        lines: kept,
    }
}

/// A line cut so that `> `, the line, `[cut]` and a line feed fit [`TEXT_BYTES_MAX`].
fn cut_line(line: &str) -> String {
    let room = TEXT_BYTES_MAX as usize - 3 - CUT_MARK.len();
    let mut end = 0;
    while end < line.len() {
        let next = end + escape_unit(&line[end..]);
        if next > room {
            break;
        }
        end = next;
    }
    let mut cut = line[..end].to_string();
    cut.push_str(CUT_MARK);
    cut
}

/// The length in bytes of the escape unit `rest` starts with (`rest` not empty): `\x` and two
/// hexadecimal digits, with the escapes that follow for the rest of that UTF-8 character; a `\`
/// with the character after it; or one character.
fn escape_unit(rest: &str) -> usize {
    let bytes = rest.as_bytes();
    if bytes[0] != b'\\' {
        return rest.chars().next().map_or(1, char::len_utf8);
    }
    let Some(lead) = hex_escape(bytes) else {
        return 1 + rest[1..].chars().next().map_or(0, char::len_utf8);
    };
    let width = match lead {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xFF => 4,
        _ => 1,
    };
    let mut length = 4;
    for _ in 1..width {
        match bytes.get(length..).and_then(hex_escape) {
            Some(0x80..=0xBF) => length += 4,
            _ => break,
        }
    }
    length
}

/// The byte `\x` and two hexadecimal digits at the start of `bytes` stand for.
fn hex_escape(bytes: &[u8]) -> Option<u8> {
    match bytes {
        [b'\\', b'x', high, low, ..] => {
            let digit = |b: u8| (b as char).to_digit(16);
            Some((digit(*high)? * 16 + digit(*low)?) as u8)
        }
        _ => None,
    }
}

/// Every line of a rendering behind `>`, after the header `text <form> <n> of <n> lines from 1,
/// <b> of <b> bytes`, without the bounds of [`text_page`] and without a `more` line: for
/// `surf <url> --as`, which prints a whole page. Each line is checked as [`render`] checks a
/// body line.
pub fn render_text_whole(
    form: Form,
    lines: impl Iterator<Item = String>,
) -> Result<String, RenderError> {
    let mut body = String::new();
    let mut count = 0u64;
    let mut bytes = 0u64;
    for line in lines {
        count += 1;
        if !is_body_line(&line) {
            return Err(RenderError::TextLine { line: count });
        }
        bytes += body_bytes(&line);
        push_body_line(&line, &mut body);
    }
    let mut out = text_header(form, count, count, 1, bytes, bytes);
    out.push('\n');
    out.push_str(&body);
    Ok(out)
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

fn is_handle_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit()
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

const BLOCKS: [Block; 7] = [
    parse_token,
    parse_did,
    parse_page,
    parse_path,
    parse_now,
    parse_notes,
    parse_text,
];

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
        let handle = if cursor.rest().starts_with(' ') && !cursor.rest().starts_with(" \"") {
            cursor.space()?;
            Some(cursor.word(is_handle_char)?.to_string())
        } else {
            None
        };
        let (argument, argument_cut) = match (&handle, cursor.rest().is_empty()) {
            (_, true) => (None, false),
            (Some(_), false) => {
                cursor.space()?;
                let label = cursor.cut_quoted(LABEL_CHARS_MAX)?;
                (Some(label.kept), label.cut)
            }
            (None, false) => (cursor.optional_quoted()?, false),
        };
        cursor.end()?;
        entries.push(PathEntry {
            sequence,
            command,
            handle,
            argument,
            argument_cut,
        });
    }
    answer.path = Some(Path { total, entries });
    Ok(())
}

fn parse_now(lines: &mut Lines<'_>, answer: &mut Answer) -> Result<(), ParseError> {
    let Some(line) = lines.take_if_starts("now ") else {
        return Ok(());
    };
    let mut cursor = line.cursor();
    cursor.at = "now ".len();
    let shown: u64 = cursor.number()?;
    cursor.literal(" of ")?;
    let total: u64 = cursor.number()?;
    cursor.end()?;
    if shown > total {
        return Err(line.error(ParseErrorKind::OutsideGrammar));
    }
    let mut elements = Vec::new();
    for _ in 0..shown {
        let Some(element_line) = lines.take_if_starts("  ") else {
            return Err(ParseError {
                line: lines.number(),
                kind: ParseErrorKind::OutsideGrammar,
            });
        };
        let mut cursor = element_line.cursor();
        cursor.at = 2;
        let handle = cursor.word(is_handle_char)?.to_string();
        cursor.space()?;
        let kind = cursor.word(is_command_char)?.to_string();
        cursor.space()?;
        let word = cursor.word(|c| c.is_ascii_lowercase() || c == '?')?;
        let visibility = Visibility::ALL
            .into_iter()
            .find(|visibility| visibility.word() == word)
            .ok_or_else(|| element_line.error(ParseErrorKind::OutsideGrammar))?;
        cursor.space()?;
        let label = cursor.cut_quoted(LABEL_CHARS_MAX)?;
        cursor.literal(" -> ")?;
        let target = cursor.cut_quoted(TARGET_CHARS_MAX)?;
        let target = match cursor.rest() {
            "" => Target::Resolved(target),
            " unresolved" => {
                cursor.at = element_line.text.len();
                Target::Unresolved(target)
            }
            _ => return Err(element_line.error(ParseErrorKind::OutsideGrammar)),
        };
        cursor.end()?;
        elements.push(Element {
            handle,
            kind,
            visibility,
            label,
            target,
        });
    }
    answer.now = Some(Now { total, elements });
    Ok(())
}

fn parse_notes(lines: &mut Lines<'_>, answer: &mut Answer) -> Result<(), ParseError> {
    while let Some(line) = lines.take_if_starts("note ") {
        if line.text == VISIBILITY_NOTE {
            answer.notes.push(Note::Visibility);
            continue;
        }
        if let Some(word) = line.text.strip_prefix("note text: ") {
            if word.starts_with("deep ") {
                let mut cursor = line.cursor();
                cursor.at = "note text: deep ".len();
                let levels: u64 = cursor.number()?;
                cursor.end()?;
                if levels == 0 {
                    return Err(line.error(ParseErrorKind::OutsideGrammar));
                }
                answer.notes.push(Note::Text(TextNote::Deep(levels)));
                continue;
            }
            let what = TextNote::WORDS
                .into_iter()
                .find(|what| what.word() == word)
                .ok_or_else(|| line.error(ParseErrorKind::OutsideGrammar))?;
            answer.notes.push(Note::Text(what));
            continue;
        }
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

fn parse_text(lines: &mut Lines<'_>, answer: &mut Answer) -> Result<(), ParseError> {
    let Some(header) = lines.take_if_starts("text ") else {
        return Ok(());
    };
    let mut cursor = header.cursor();
    cursor.at = "text ".len();
    let word = cursor.word(|c| c.is_ascii_lowercase())?;
    let form = Form::ALL
        .into_iter()
        .find(|form| form.word() == word)
        .ok_or_else(|| header.error(ParseErrorKind::OutsideGrammar))?;
    cursor.space()?;
    let shown: u64 = cursor.number()?;
    cursor.literal(" of ")?;
    let total: u64 = cursor.number()?;
    cursor.literal(" lines from ")?;
    let first: u64 = cursor.number_before(',')?;
    cursor.literal(", ")?;
    let bytes: u64 = cursor.number()?;
    cursor.literal(" of ")?;
    let bytes_total: u64 = cursor.number()?;
    cursor.literal(" bytes")?;
    cursor.end()?;
    let outside = || header.error(ParseErrorKind::OutsideGrammar);
    if shown > TEXT_LINES_MAX as u64
        || bytes > TEXT_BYTES_MAX
        || !counts_hold(shown, total, first, bytes, bytes_total)
    {
        return Err(outside());
    }
    let mut body = Vec::new();
    for _ in 0..shown {
        let Some(line) = lines.next() else {
            return Err(ParseError {
                line: lines.last_number(),
                kind: ParseErrorKind::OutsideGrammar,
            });
        };
        let content = match line.text {
            ">" => "",
            text => match text.strip_prefix("> ") {
                Some(rest) if !rest.is_empty() && is_body_line(rest) => rest,
                _ => return Err(line.error(ParseErrorKind::OutsideGrammar)),
            },
        };
        body.push(content.to_string());
    }
    if body.iter().map(|line| body_bytes(line)).sum::<u64>() != bytes {
        return Err(outside());
    }
    if let Some(expected) = more_line(form, first, shown, total) {
        match lines.next() {
            Some(line) if line.text == expected => {}
            Some(line) => return Err(line.error(ParseErrorKind::OutsideGrammar)),
            None => {
                return Err(ParseError {
                    line: lines.last_number(),
                    kind: ParseErrorKind::OutsideGrammar,
                });
            }
        }
    }
    answer.text = Some(Text {
        form,
        first,
        total,
        bytes_total,
        lines: body,
    });
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

    /// The number of the last line, where a block that ends too early is refused.
    fn last_number(&self) -> usize {
        self.lines.len().max(1)
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

    /// A number as [`Cursor::number`] reads it, ending at `end` instead of a space.
    fn number_before<T: std::str::FromStr>(&mut self, end: char) -> Result<T, ParseError> {
        let rest = self.rest();
        let digits = rest.split(end).next().unwrap_or("");
        if !is_word(digits, |c| c.is_ascii_digit()) || (digits.len() > 1 && digits.starts_with('0'))
        {
            return Err(self.outside());
        }
        self.at += digits.len();
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

    /// One field with a length limit of `limit` characters: a quoted field, then `...` when the
    /// value was cut, read back by [`unquote_cut`].
    fn cut_quoted(&mut self, limit: usize) -> Result<Cut, ParseError> {
        let rest = self.rest();
        if !rest.starts_with('"') {
            return Err(self.outside());
        }
        let mut end = match rest[1..].find('"') {
            Some(close) => close + 2,
            None => rest.len(),
        };
        if rest[end..].starts_with("...") {
            end += 3;
        }
        let value = unquote_cut(&rest[..end], limit)
            .map_err(|error| self.line.error(ParseErrorKind::Field(error)))?;
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
