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
/// `{"id","state","address","title","opened_at","last_action_at","ended_at","signal","page"}`.
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
    /// The name of the signal its process ended by, such as `SIGXCPU`, when `surfd` did not
    /// kill it; `None` otherwise.
    pub signal: Option<String>,
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
/// `{"handle","kind","visibility","label","label_cut","target","target_cut","unresolved","text","state"}`.
/// A link has the members from `label` to `unresolved`, and no `text` and no `state`. A control
/// or a button (`story:view-controls-and-changes`) has `label`, `label_cut`, `text` (its line as
/// the answer writes it) and `state`, and no target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NowElement {
    /// Its handle (`l1`).
    pub handle: String,
    /// The kind word of its line (`link`).
    pub kind: String,
    /// The answer's visibility word: `vis`, `hid` or `?`.
    pub visibility: String,
    /// The label, as the answer cut it.
    pub label: Option<String>,
    /// Whether the label was cut.
    pub label_cut: Option<bool>,
    /// A link's target, as the answer cut it.
    pub target: Option<String>,
    /// Whether the target was cut.
    pub target_cut: Option<bool>,
    /// Whether the target did not resolve and is the `href` as the page wrote it.
    pub unresolved: Option<bool>,
    /// The line of an element that is not a link, without its indentation.
    pub text: Option<String>,
    /// The state of a control or a button; `None` for a link.
    pub state: Option<ControlState>,
}

/// The state of a control or a button on its `now` line,
/// `{"value","disabled","checked","required","readonly","submits"}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlState {
    /// `= <value>` of the line; `None` for a checkbox, a radio button, a button and a select with
    /// no option selected.
    pub value: Option<ControlValue>,
    /// `disabled`.
    pub disabled: bool,
    /// `checked`.
    pub checked: bool,
    /// `required`.
    pub required: bool,
    /// `readonly`.
    pub readonly: bool,
    /// What a submit button with a form owner submits.
    pub submits: Option<Submits>,
}

/// A control's value as the answer prints it, `{"kind","text","cut"}`: a quoted value
/// (`"kind":"text"`), or a password's `(set)` or `(empty)` (`"kind":"set"`, `"kind":"empty"`,
/// `text` and `cut` `null`), never a password's value (decision D22).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlValue {
    /// A quoted value: a text control's value, or the label of a select's selected option.
    Text {
        /// The value as the answer cut it.
        text: String,
        /// Whether it was cut.
        cut: bool,
    },
    /// `(set)`: a password control that holds a value.
    Set,
    /// `(empty)`: a password control with no value.
    Empty,
}

/// `submits F<n> <method> "<action>"` of a button line, `{"form","method","action","action_cut"}`;
/// `action` and `action_cut` `null` for `unresolved`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Submits {
    /// The form's handle (`F1`).
    pub form: String,
    /// The method word (`post`).
    pub method: String,
    /// The action, as the answer cut it; `None` when it does not resolve.
    pub action: Option<String>,
    /// Whether the action was cut.
    pub action_cut: Option<bool>,
}

/// The `changed` member of an `answer` entry: what the action changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Changed {
    /// `{"kind":"controls","lines":[…]}`: `changed <n>` and its `~` lines; no line for
    /// `changed 0`.
    Controls(Vec<ControlChange>),
    /// `{"kind":"page","from","to","texts":[…],"more"}`: `changed page <old> -> <new>` and its
    /// `+ text` lines.
    Page {
        /// The address of the previous answer's page; `None` for `none`.
        from: Option<String>,
        /// The address of this answer's page; `None` for `none`.
        to: Option<String>,
        /// The new texts shown, each `{"text","cut"}`.
        texts: Vec<ChangedText>,
        /// How many new texts are not shown (`+ text more <n>`), 0 without that line.
        more: u64,
    },
}

/// One `+ text` line, `{"text","cut"}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangedText {
    /// The text, as the answer cut it.
    pub text: String,
    /// Whether it was cut.
    pub cut: bool,
}

/// One `~` line, `{"handle","what","before","after"}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlChange {
    /// The control's handle (`f1`).
    pub handle: String,
    /// What changed, with its two sides.
    pub change: Change,
}

/// What changed of a control: `what` is `value`, `checked` or `selected`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// `value <before> -> <after>`, each a [`ControlValue`].
    Value {
        /// The value before.
        before: ControlValue,
        /// The value after.
        after: ControlValue,
    },
    /// `checked <before> -> <after>`, each written `true` or `false`.
    Checked {
        /// Checked before.
        before: bool,
        /// Checked after.
        after: bool,
    },
    /// `selected "<before>" -> "<after>"`, each a [`ControlValue::Text`].
    Selected {
        /// The labels before.
        before: ControlValue,
        /// The labels after.
        after: ControlValue,
    },
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
    /// `"kind":"answer"`, then `"page"`, `"path"`, `"now"` and `"changed"`.
    Answer {
        /// The page line.
        page: Option<PageLine>,
        /// The path.
        path: Option<Path>,
        /// The `now` block; `None` when the answer has none.
        now: Option<Now>,
        /// The `changed` block; `None` when the answer has none.
        changed: Option<Changed>,
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
