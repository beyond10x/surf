//! The view's own values, one type per JSON shape of `docs/view-stream.md` ("The JSON"), which
//! [`crate::to_json`] writes. Every string is the exact value, read through
//! `surf_answer::unquote` or `surf_answer::parse` by the caller (decision D26); `None` is a
//! value that is absent or that those refused, written `null`.
//!
//! No type has a member for a token (decision D21).

/// The `page` line of an answer, as `surf_answer::parse` reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageLine {
    /// `{"kind":"none"}`, for `page none`.
    None,
    /// `{"kind":"loaded","status","media_type","address","title"}`.
    Loaded {
        /// The response status, written as a number.
        status: u16,
        /// The media type.
        media_type: String,
        /// The address of the page.
        address: String,
        /// The title, `null` when the answer has none.
        title: Option<String>,
    },
    /// `{"kind":"failed","reason","address"}`.
    Failed {
        /// Why the load failed, such as `Unreachable`.
        reason: String,
        /// The address that failed.
        address: String,
    },
}

/// The state of a listed session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState {
    /// `open`.
    Open,
    /// `closed`.
    Closed,
    /// `died`.
    Died,
}

/// A session summary,
/// `{"id","state","address","title","opened_at","last_action_at","ended_at","page"}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    /// The session identity.
    pub id: String,
    /// Its state.
    pub state: SessionState,
    /// The address as the session last reported it.
    pub address: Option<String>,
    /// The title as the session last reported it.
    pub title: Option<String>,
    /// When it was opened: milliseconds since the Unix epoch.
    pub opened_at: u64,
    /// The `kept_at` of its newest action entry.
    pub last_action_at: Option<u64>,
    /// When it ended; `None` while it is open.
    pub ended_at: Option<u64>,
    /// The page line of its newest kept answer entry.
    pub page: Option<PageLine>,
}

/// The session list, the 200 of `/api/sessions`: `{"newest","sessions"}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionList {
    /// The highest number across sessions kept so far, 0 before the first entry.
    pub newest: u64,
    /// Every listed session, oldest `opened_at` first.
    pub sessions: Vec<Summary>,
}

/// The session detail, the 200 of `/api/sessions/<id>`:
/// `{"session","dropped","log_dropped","unreadable","entries"}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Detail {
    /// The summary.
    pub session: Summary,
    /// Entries of this session dropped by any bound.
    pub dropped: u64,
    /// Whether the bound over all sessions dropped its log whole.
    pub log_dropped: bool,
    /// Entries holding a value that `surf_answer::unquote` or `surf_answer::parse` refused.
    pub unreadable: u64,
    /// The kept entries in their order.
    pub entries: Vec<Entry>,
}

/// The action of an `action` entry, `{"sequence","command","argument"}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    /// Its sequence number.
    pub sequence: Option<u64>,
    /// Its command.
    pub command: Option<String>,
    /// Its argument, the exact text; `None` when it has none.
    pub argument: Option<String>,
}

/// One action of the `path` of an `answer` entry,
/// `{"sequence","command","handle","argument","argument_cut"}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathAction {
    /// Its sequence number.
    pub sequence: u64,
    /// Its command.
    pub command: String,
    /// The handle of the element it took (`l1`); `None` for an action on no element.
    pub handle: Option<String>,
    /// Its argument, the exact text: an address, or with a handle the element's label as the
    /// answer cut it; `None` when it has none.
    pub argument: Option<String>,
    /// Whether the label was cut at 60 characters.
    pub argument_cut: bool,
}

/// The `path` of an `answer` entry, `{"shown","total","actions"}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    /// How many actions the answer shows.
    pub shown: u64,
    /// How many actions the session has taken.
    pub total: u64,
    /// The actions shown, oldest first.
    pub actions: Vec<PathAction>,
}

/// The `now` of an `answer` entry, `{"shown","total","elements","visibility_note"}`: the
/// actionable elements of the current document, the first ones in document order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Now {
    /// How many elements the answer shows.
    pub shown: u64,
    /// How many the document has.
    pub total: u64,
    /// The elements shown, one per line of the block.
    pub elements: Vec<NowElement>,
    /// `hidden attribute only` when the answer holds that visibility note.
    pub visibility_note: Option<String>,
}

/// One element of a `now`,
/// `{"handle","kind","visibility","label","label_cut","target","target_cut","unresolved","text"}`.
/// A link has the members from `label` to `unresolved` and no `text`; a line of a kind the view
/// does not know has `text`, the line as the answer writes it, and none of the others.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NowElement {
    /// Its handle (`l1`).
    pub handle: String,
    /// The kind word of its line (`link`).
    pub kind: String,
    /// The answer's visibility word: `vis`, `hid` or `?`.
    pub visibility: String,
    /// A link's label, as the answer cut it.
    pub label: Option<String>,
    /// Whether the label was cut.
    pub label_cut: Option<bool>,
    /// A link's target, as the answer cut it.
    pub target: Option<String>,
    /// Whether the target was cut.
    pub target_cut: Option<bool>,
    /// Whether the target did not resolve and is the `href` as the page wrote it.
    pub unresolved: Option<bool>,
    /// The line of a kind the view does not know, without its indentation.
    pub text: Option<String>,
}

/// What follows the five members every entry starts with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// `"kind":"action"`, then the action's members.
    Action(Action),
    /// `"kind":"event"`, then `"name"`, `"unmapped"` and `"fields"`.
    Event {
        /// The qualified event name of `spec/`.
        name: Option<String>,
        /// Whether the session process does not map this event yet; `fields` is then empty.
        unmapped: bool,
        /// Every field by its `spec/` name, in the order the event declares them; `None` for an
        /// absent value.
        fields: Vec<(String, Option<String>)>,
    },
    /// `"kind":"answer"`, then `"page"`, `"path"` and `"now"`.
    Answer {
        /// The page line.
        page: Option<PageLine>,
        /// The path.
        path: Option<Path>,
        /// The `now` block; `None` when the answer has none.
        now: Option<Now>,
    },
}

/// One entry, `{"number","session","index","kept_at","kind",…}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Its number across all sessions (decision D23), the `id:` of the stream.
    pub number: u64,
    /// The session identity.
    pub session: String,
    /// Its number within the session, from 1.
    pub index: u64,
    /// When `surfd` kept it: milliseconds since the Unix epoch.
    pub kept_at: u64,
    /// Its kind and what follows it.
    pub kind: EntryKind,
}

/// The data of the stream's `reset` event, `{"newest"}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reset {
    /// The newest number.
    pub newest: u64,
}
