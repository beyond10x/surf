//! The live view's data: the [`surf_view::Source`] over the session list and the log, which
//! `surfd` serves on 127.0.0.1 (`docs/view-stream.md`). Each route reads what it needs in one
//! hold of the daemon's lock ([`Tables::hold`]) and converts it after: an address, a title, an
//! action's argument and an event field through [`surf_answer::unquote`], an answer entry
//! through [`surf_answer::parse`] (decisions D20 and D26); a value they refuse is `null`, the
//! entry is kept and counted in `unreadable`. Nothing here reads or writes a token. Of an
//! answer's `now` block a link goes out with its label and target, and a line of any other kind
//! with its kind and its text as [`surf_answer::render`] writes it: there is no `match` on the
//! kind, so a story that adds one changes nothing here.
//!
//! - `/api/sessions`: the newest number and the list, in one hold; per session its newest kept
//!   action entry (`last_action_at`) and answer entry (`page`), read in place ([`Log::newest_of`]).
//! - `/api/sessions/<id>`: the summary and the kept entries of a listed session.
//! - `/events`: a reader over a [`Subscription`]. It answers [`TooOld`] when an entry above the
//!   number was dropped, or when the number is more than [`surf_view::Limits::queue_entries`]
//!   below the newest, so that a replay always fits the stream's queue. A stream from the newest
//!   number reads that number and subscribes in one hold ([`Source::read_newest`]), so no entry
//!   kept meanwhile makes it `TooOld`. After each live answer
//!   entry (not a replayed one) it yields a `session` item of its session with the same number;
//!   and at each wait, with no
//!   entry pending, it compares the list with the one it knows and yields a `session` item, with
//!   the newest number, for a session listed that it did not know or one that became `closed`
//!   or `died`. Entries are kept under the same lock, so no entry is in flight while it
//!   compares, and the numbers on a stream never go down. A subscription that was disconnected
//!   (its queue full) ends the stream.
//!
//! A reader of the view is not a connection of `surfd.sock`: it keeps `surfd` from no idle exit.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::mpsc::{RecvTimeoutError, TryRecvError};
use std::time::{Duration, SystemTime};

use surf_answer::{Element, Note, Page, Target};
use surf_view::model::{
    Action, Detail, Entry, EntryKind, Now, NowElement, PageLine, Path, PathAction, SessionList,
    SessionState, Summary,
};
use surf_view::{EventKind, Item, Next, Reader, Source, TooOld, to_json};

use crate::link::{self, EntryKind as Kind};
use crate::log::{self, Log, Subscription};
use crate::sessions::{Session, Sessions, State};

/// What the view reads of `surfd`'s state: the session list and the log, together, under one
/// hold of the lock that guards both.
pub(crate) trait Tables: Send + Sync + 'static {
    /// Call `read` with the list and the log while the lock is held.
    fn hold<R>(&self, read: impl FnOnce(&Sessions, &mut Log) -> R) -> R;
}

/// The view's [`Source`] over `tables`.
pub(crate) struct View<T> {
    tables: Arc<T>,
    /// [`surf_view::Limits::queue_entries`]: a replay longer than this is answered `reset`.
    queue_entries: u64,
}

impl<T: Tables> View<T> {
    pub(crate) fn new(tables: Arc<T>, queue_entries: usize) -> Self {
        View {
            tables,
            queue_entries: u64::try_from(queue_entries).unwrap_or(u64::MAX),
        }
    }
}

/// What a summary needs, copied in one hold and converted after.
struct Raw {
    session: Session,
    last_action_at: Option<u64>,
    /// The lines of its newest kept answer entry.
    answer: Option<String>,
}

impl Raw {
    fn read(session: &Session, log: &Log) -> Raw {
        Raw {
            session: session.clone(),
            last_action_at: log
                .newest_of(&session.id, Kind::Action)
                .map(|entry| entry.kept_at),
            answer: log
                .newest_of(&session.id, Kind::Answer)
                .map(|entry| entry.text.clone()),
        }
    }

    fn summary(&self) -> Summary {
        let session = &self.session;
        let value = |quoted: &Option<link::Quoted>| {
            quoted
                .as_ref()
                .and_then(|quoted| surf_answer::unquote(quoted.as_str()).ok())
        };
        Summary {
            id: session.id.clone(),
            state: state(session.state),
            address: value(&session.address),
            title: value(&session.title),
            opened_at: millis(session.opened),
            last_action_at: self.last_action_at,
            ended_at: session.ended.map(millis),
            page: self
                .answer
                .as_deref()
                .and_then(|text| surf_answer::parse(text).ok())
                .and_then(|answer| answer.page)
                .map(page_line),
        }
    }
}

fn state(state: State) -> SessionState {
    match state {
        State::Open => SessionState::Open,
        State::Closed => SessionState::Closed,
        State::Died => SessionState::Died,
    }
}

/// Milliseconds since the Unix epoch, 0 for a time before it.
fn millis(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

fn page_line(page: Page) -> PageLine {
    match page {
        Page::None => PageLine::None,
        Page::Loaded {
            status,
            media_type,
            address,
            title,
        } => PageLine::Loaded {
            status,
            media_type,
            address,
            title,
        },
        Page::Failed { reason, address } => PageLine::Failed { reason, address },
    }
}

/// The kind word of a link's `now` line. Every other kind goes out as its line's text.
const LINK: &str = "link";

/// The lines `surf_answer::render` writes for `now` and `notes` alone, without the head line;
/// `None` when it refuses them. A `now` line and a note are written here as the answer writes
/// them, never by `surfd` (D20).
fn rendered(session: &str, now: Option<surf_answer::Now>, notes: Vec<Note>) -> Option<String> {
    let mut answer = surf_answer::Answer::new(surf_answer::Head {
        session: session.to_string(),
        state: surf_answer::SessionState::Open,
    });
    answer.now = now;
    answer.notes = notes;
    let text = surf_answer::render(&answer).ok()?;
    let (_, rest) = text.split_once('\n')?;
    Some(rest.to_string())
}

/// The words after `note visibility: ` of the note the answer writes, `hidden attribute only`.
fn visibility_note_words(session: &str) -> Option<String> {
    let line = rendered(session, None, vec![Note::Visibility])?;
    let line = line.strip_suffix('\n').unwrap_or(&line);
    line.strip_prefix("note visibility: ").map(str::to_string)
}

/// The view's element of one `now` line. A link has its label and target; a line of any other
/// kind (a control, a button, a form, which later stories add) goes out with its kind and its
/// text, the line as `surf_answer::render` writes it, so that a new kind changes nothing here.
fn now_element(session: &str, element: Element) -> NowElement {
    let handle = element.handle.clone();
    let kind = element.kind.clone();
    let visibility = element.visibility.word().to_string();
    let link = if element.kind == LINK {
        if let Target::Resolved(target) = &element.target {
            Some((target.clone(), false))
        } else if let Target::Unresolved(target) = &element.target {
            Some((target.clone(), true))
        } else {
            None
        }
    } else {
        None
    };
    if let Some((target, unresolved)) = link {
        return NowElement {
            handle,
            kind,
            visibility,
            label: Some(element.label.kept),
            label_cut: Some(element.label.cut),
            target: Some(target.kept),
            target_cut: Some(target.cut),
            unresolved: Some(unresolved),
            text: None,
        };
    }
    let now = surf_answer::Now {
        total: 1,
        elements: vec![element],
    };
    let text = rendered(session, Some(now), Vec::new()).and_then(|lines| {
        let line = lines.lines().nth(1)?;
        line.strip_prefix("  ").map(str::to_string)
    });
    NowElement {
        handle,
        kind,
        visibility,
        label: None,
        label_cut: None,
        target: None,
        target_cut: None,
        unresolved: None,
        text,
    }
}

/// The view's entry of a kept entry; each value it cannot read is `None` and counted in
/// `unreadable`.
fn entry(kept: &log::Entry, unreadable: &mut u64) -> Entry {
    /// The exact value of a quoted field, or `None`, noted in `refused`.
    fn unquote(field: &str, refused: &mut bool) -> Option<String> {
        let value = surf_answer::unquote(field).ok();
        *refused |= value.is_none();
        value
    }
    let mut refused = false;
    let kind = match kept.kind {
        Kind::Action => match link::read_action(&kept.text) {
            Some(read) => {
                let sequence = read.sequence.parse().ok();
                let argument = read.argument.and_then(|field| unquote(field, &mut refused));
                refused |= sequence.is_none();
                EntryKind::Action(Action {
                    sequence,
                    command: Some(read.command.to_string()),
                    argument,
                })
            }
            None => {
                refused = true;
                EntryKind::Action(Action {
                    sequence: None,
                    command: None,
                    argument: None,
                })
            }
        },
        Kind::Event => match link::read_event(&kept.text) {
            Some(read) => EntryKind::Event {
                name: Some(read.name.to_string()),
                unmapped: read.unmapped,
                fields: read
                    .fields
                    .iter()
                    .map(|(name, value)| {
                        let value = value.and_then(|field| unquote(field, &mut refused));
                        (name.to_string(), value)
                    })
                    .collect(),
            },
            None => {
                refused = true;
                EntryKind::Event {
                    name: None,
                    unmapped: false,
                    fields: Vec::new(),
                }
            }
        },
        Kind::Answer => match surf_answer::parse(&kept.text) {
            Ok(answer) => {
                let visibility_note = answer.notes.contains(&Note::Visibility);
                EntryKind::Answer {
                    page: answer.page.map(page_line),
                    path: answer.path.map(|path| Path {
                        shown: path.entries.len() as u64,
                        total: path.total,
                        actions: path
                            .entries
                            .into_iter()
                            .map(|action| PathAction {
                                sequence: action.sequence,
                                command: action.command,
                                handle: action.handle,
                                argument: action.argument,
                                argument_cut: action.argument_cut,
                            })
                            .collect(),
                    }),
                    now: answer.now.map(|now| Now {
                        shown: now.elements.len() as u64,
                        total: now.total,
                        elements: now
                            .elements
                            .into_iter()
                            .map(|element| now_element(&kept.session, element))
                            .collect(),
                        visibility_note: visibility_note
                            .then(|| visibility_note_words(&kept.session))
                            .flatten(),
                    }),
                }
            }
            Err(_) => {
                refused = true;
                EntryKind::Answer {
                    page: None,
                    path: None,
                    now: None,
                }
            }
        },
    };
    if refused {
        *unreadable += 1;
    }
    Entry {
        number: kept.number,
        session: kept.session.clone(),
        index: kept.index,
        kept_at: kept.kept_at,
        kind,
    }
}

impl<T: Tables> Source for View<T> {
    fn sessions(&self) -> Vec<u8> {
        let (newest, raws) = self.tables.hold(|sessions, log| {
            let raws: Vec<Raw> = sessions
                .iter()
                .map(|session| Raw::read(session, log))
                .collect();
            (log.newest(), raws)
        });
        to_json(&SessionList {
            newest,
            sessions: raws.iter().map(Raw::summary).collect(),
        })
    }

    fn session(&self, id: &str) -> Option<Vec<u8>> {
        let (raw, kept) = self.tables.hold(|sessions, log| {
            let session = sessions.get(id)?;
            Some((
                Raw::read(session, log),
                log.session_entries(id).unwrap_or_default(),
            ))
        })?;
        let mut unreadable = 0;
        let entries = kept
            .entries
            .iter()
            .map(|kept| entry(kept, &mut unreadable))
            .collect();
        Some(to_json(&Detail {
            session: raw.summary(),
            dropped: kept.dropped,
            log_dropped: kept.log_dropped,
            unreadable,
            entries,
        }))
    }

    fn newest(&self) -> u64 {
        self.tables.hold(|_, log| log.newest())
    }

    fn read_from(&self, number: u64) -> Result<Box<dyn Reader>, TooOld> {
        self.open(Some(number)).map(|(_, reader)| reader)
    }

    fn read_newest(&self) -> Result<(u64, Box<dyn Reader>), TooOld> {
        self.open(None)
    }
}

impl<T: Tables> View<T> {
    /// A reader from `from`, or from the newest number read in the same hold, with that number.
    fn open(&self, from: Option<u64>) -> Result<(u64, Box<dyn Reader>), TooOld> {
        let most = self.queue_entries;
        let (number, subscription, replay_until, known) = self.tables.hold(|sessions, log| {
            let newest = log.newest();
            let number = from.unwrap_or(newest);
            if newest.saturating_sub(number) > most {
                return Err(TooOld);
            }
            let subscription = log.subscribe_after(number).map_err(|_| TooOld)?;
            Ok((number, subscription, newest, known_states(sessions)))
        })?;
        let reader: Box<dyn Reader> = Box::new(Stream {
            tables: Arc::clone(&self.tables),
            subscription,
            replay_until,
            known,
            pending: VecDeque::new(),
        });
        Ok((number, reader))
    }
}

fn known_states(sessions: &Sessions) -> HashMap<String, State> {
    sessions
        .iter()
        .map(|session| (session.id.clone(), session.state))
        .collect()
}

/// The reader of one stream.
struct Stream<T> {
    tables: Arc<T>,
    subscription: Subscription,
    /// The newest number when the stream opened: an entry up to it is replayed, and a replayed
    /// answer entry is followed by no `session` item.
    replay_until: u64,
    /// The state of each session as this stream last told it, or as it was listed when the
    /// stream opened.
    known: HashMap<String, State>,
    /// `session` items to yield before anything else.
    pending: VecDeque<Item>,
}

/// What one look under the lock found.
enum Polled {
    /// An entry, and for an answer entry of a listed session that session's summary; boxed, as
    /// the two take about 250 bytes and the other variants a few words.
    Entry(Box<(log::Entry, Option<Raw>)>),
    /// No entry now; the sessions whose change the stream had not told, with the newest number.
    Quiet(u64, Vec<Raw>),
    /// The subscription was disconnected.
    End,
}

/// The summary of `entry`'s session after a live answer entry, noted as told; `None` after a
/// replayed one (up to `replay_until`), as `session` items are never replayed.
fn after_answer(
    entry: &log::Entry,
    replay_until: u64,
    sessions: &Sessions,
    log: &Log,
    known: &mut HashMap<String, State>,
) -> Option<Raw> {
    if entry.kind != Kind::Answer || entry.number <= replay_until {
        return None;
    }
    let session = sessions.get(&entry.session)?;
    known.insert(session.id.clone(), session.state);
    Some(Raw::read(session, log))
}

/// The sessions listed that `known` lacks, and those that became `closed` or `died`, noted as
/// told.
fn changes(sessions: &Sessions, log: &Log, known: &mut HashMap<String, State>) -> Vec<Raw> {
    let mut changed = Vec::new();
    for session in sessions.iter() {
        let told = known.insert(session.id.clone(), session.state);
        let ended = matches!(session.state, State::Closed | State::Died);
        if told.is_none() || (ended && told != Some(session.state)) {
            changed.push(Raw::read(session, log));
        }
    }
    known.retain(|id, _| sessions.get(id).is_some());
    changed
}

fn entry_item(kept: &log::Entry) -> Item {
    let mut unreadable = 0;
    Item {
        number: kept.number,
        kind: EventKind::Entry,
        data: to_json(&entry(kept, &mut unreadable)),
    }
}

fn session_item(number: u64, raw: &Raw) -> Item {
    Item {
        number,
        kind: EventKind::Session,
        data: to_json(&raw.summary()),
    }
}

impl<T: Tables> Stream<T> {
    /// The item of `kept`, with the `session` item after an answer entry queued.
    fn entry(&mut self, kept: &log::Entry, raw: Option<Raw>) -> Next {
        if let Some(raw) = raw {
            self.pending.push_back(session_item(kept.number, &raw));
        }
        Next::Item(entry_item(kept))
    }
}

impl<T: Tables> Reader for Stream<T> {
    fn next(&mut self, wait: Duration) -> Next {
        if let Some(item) = self.pending.pop_front() {
            return Next::Item(item);
        }
        let Stream {
            tables,
            subscription,
            replay_until,
            known,
            ..
        } = self;
        let replay_until = *replay_until;
        let polled = tables.hold(|sessions, log| match subscription.try_next() {
            Ok(kept) => {
                let raw = after_answer(&kept, replay_until, sessions, log, known);
                Polled::Entry(Box::new((kept, raw)))
            }
            Err(TryRecvError::Empty) => Polled::Quiet(log.newest(), changes(sessions, log, known)),
            Err(TryRecvError::Disconnected) => Polled::End,
        });
        match polled {
            Polled::Entry(found) => {
                let (kept, raw) = *found;
                self.entry(&kept, raw)
            }
            Polled::End => Next::End,
            Polled::Quiet(newest, changed) if !changed.is_empty() => {
                self.pending
                    .extend(changed.iter().map(|raw| session_item(newest, raw)));
                self.pending.pop_front().map_or(Next::Idle, Next::Item)
            }
            Polled::Quiet(..) => match self.subscription.next_within(wait) {
                Ok(kept) => {
                    let Stream { tables, known, .. } = self;
                    let raw = tables.hold(|sessions, log| {
                        after_answer(&kept, replay_until, sessions, log, known)
                    });
                    self.entry(&kept, raw)
                }
                Err(RecvTimeoutError::Timeout) => Next::Idle,
                Err(RecvTimeoutError::Disconnected) => Next::End,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, PoisonError};
    use std::time::UNIX_EPOCH;

    use super::*;
    use crate::log::Bounds;

    const ID: &str = "aaaaaaaa-0000-4000-8000-000000000001";

    /// The list and the log behind one lock, without a daemon.
    struct Fixed(Mutex<(Sessions, Log)>);

    impl Tables for Fixed {
        fn hold<R>(&self, read: impl FnOnce(&Sessions, &mut Log) -> R) -> R {
            let mut held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let (sessions, log) = &mut *held;
            read(sessions, log)
        }
    }

    fn fixed(bounds: Bounds) -> Arc<Fixed> {
        let at = UNIX_EPOCH + Duration::from_millis(1_795_226_400_000);
        let session = Session {
            id: ID.to_string(),
            state: State::Open,
            opened: at,
            last_action: at,
            ended: None,
            pid: 1,
            address: None,
            title: None,
            log_dropped: false,
        };
        Arc::new(Fixed(Mutex::new((
            Sessions {
                list: vec![session],
            },
            Log::new(bounds),
        ))))
    }

    fn keep(tables: &Fixed, entry: link::Entry) {
        tables.hold(|_, log| log.keep(ID, entry, 1));
    }

    fn action(sequence: u64) -> link::Entry {
        link::Entry::line(&format!("action {sequence} go \"http://a.example/\"")).unwrap()
    }

    #[test]
    fn case_25_a_disconnected_subscription_ends_the_stream() {
        let tables = fixed(Bounds {
            reader_queue: 2,
            ..Bounds::default()
        });
        let view = View::new(Arc::clone(&tables), 256);
        let Ok(mut reader) = view.read_from(0) else {
            panic!("a reader from 0");
        };
        for sequence in 1..=3 {
            keep(&tables, action(sequence));
        }
        let mut numbers = Vec::new();
        loop {
            match reader.next(Duration::from_millis(10)) {
                Next::Item(item) => {
                    assert_eq!(item.kind, EventKind::Entry);
                    numbers.push(item.number);
                }
                Next::Idle => panic!("idle after {numbers:?}: the stream did not end"),
                Next::End => break,
            }
        }
        assert_eq!(numbers, [1, 2], "two entry items, then the end");
    }

    /// F1: with more than `queue_entries` entries kept after a number, `read_from` that number
    /// is `TooOld`, and `read_newest` gives the newest number with a reader live after it.
    #[test]
    fn read_newest_reads_the_number_and_subscribes_in_one_hold() {
        let tables = fixed(Bounds::default());
        let view = View::new(Arc::clone(&tables), 256);
        let before = view.newest();
        for sequence in 1..=300 {
            keep(&tables, action(sequence));
        }
        assert!(view.read_from(before).is_err(), "300 kept after {before}");
        let Ok((newest, mut reader)) = view.read_newest() else {
            panic!("a reader from the newest number");
        };
        assert_eq!(newest, before + 300);
        assert!(matches!(
            reader.next(Duration::from_millis(10)),
            Next::Idle
        ));
        keep(&tables, action(301));
        match reader.next(Duration::from_secs(10)) {
            Next::Item(item) => assert_eq!(item.number, newest + 1),
            other => panic!("the next entry live, not {other:?}"),
        }
    }

    #[test]
    fn case_26_what_parse_or_unquote_refuses_is_null_and_counted() {
        let tables = fixed(Bounds::default());
        keep(&tables, action(1));
        keep(
            &tables,
            link::Entry::line("event surf.agent.ActionRecorded action_id=\"x\" url=").unwrap(),
        );
        keep(
            &tables,
            link::Entry::line("event surf.render.FramePainted unmapped").unwrap(),
        );
        keep(&tables, link::Entry::answer("not an answer\n"));
        let view = View::new(Arc::clone(&tables), 256);
        let detail = String::from_utf8(view.session(ID).expect("listed")).unwrap();
        assert!(detail.contains(r#""unreadable":1,"#), "{detail}");
        assert!(
            detail.contains(r#""fields":{"action_id":"x","url":null}"#),
            "{detail}"
        );
        assert!(
            detail.contains(r#""name":"surf.render.FramePainted","unmapped":true,"fields":{}"#),
            "{detail}"
        );
        assert!(
            detail.contains(
                r#""index":4,"kept_at":1,"kind":"answer","page":null,"path":null,"now":null}"#
            ),
            "{detail}"
        );
        let entries = tables.hold(|_, log| log.session_entries(ID).unwrap());
        assert_eq!(entries.entries.len(), 4, "the refused answer entry is kept");
        assert!(
            detail.contains(r#""page":null},"dropped""#),
            "the summary's page of an answer parse refuses: {detail}"
        );
        assert_eq!(view.session("bbbbbbbb-0000-4000-8000-000000000002"), None);
    }

    /// `story:view-now-panel`: the title, the handle of a path action and the `now` block reach
    /// the view; a `now` line of a kind the view does not know goes out with its kind and its
    /// text, so a story that adds a kind changes nothing here.
    #[test]
    fn now_block_title_and_handle_reach_the_view_and_other_kinds_go_out_as_text() {
        let tables = fixed(Bounds::default());
        keep(&tables, action(2));
        keep(
            &tables,
            link::Entry::answer(concat!(
                "surf 1 session aaaaaaaa-0000-4000-8000-000000000001 open\n",
                "page 200 \"text/html\" \"http://a.example/next\" \"Next <page>\"\n",
                "path 2 of 2\n",
                "  1 go \"http://a.example/\"\n",
                "  2 follow l1 \"012345678901234567890123456789012345678901234567890123456789\"...\n",
                "now 3 of 4\n",
                "  l1 link ? \"Back\" -> \"/\"\n",
                "  f1 text hid \"Email\" = \"x\"\n",
                "  b1 button ? \"Sign in\" submits F1 post \"/session\"\n",
                "note visibility: hidden attribute only\n",
            )),
        );
        let view = View::new(Arc::clone(&tables), 256);
        let detail = String::from_utf8(view.session(ID).expect("listed")).unwrap();
        let expected = concat!(
            r#""kind":"answer","page":{"kind":"loaded","status":200,"#,
            r#""media_type":"text/html","address":"http://a.example/next","#,
            "\"title\":\"Next \\u003cpage\\u003e\"},",
            r#""path":{"shown":2,"total":2,"actions":["#,
            r#"{"sequence":1,"command":"go","handle":null,"argument":"http://a.example/","#,
            r#""argument_cut":false},"#,
            r#"{"sequence":2,"command":"follow","handle":"l1","argument":"012345678901234567890123456789012345678901234567890123456789","#,
            r#""argument_cut":true}]},"#,
            r#""now":{"shown":3,"total":4,"elements":["#,
            r#"{"handle":"l1","kind":"link","visibility":"?","label":"Back","label_cut":false,"#,
            r#""target":"/","target_cut":false,"unresolved":false,"text":null},"#,
            r#"{"handle":"f1","kind":"text","visibility":"hid","label":null,"label_cut":null,"#,
            r#""target":null,"target_cut":null,"unresolved":null,"#,
            "\"text\":\"f1 text hid \\u0022Email\\u0022 = \\u0022x\\u0022\"},",
            r#"{"handle":"b1","kind":"button","visibility":"?","label":null,"label_cut":null,"#,
            r#""target":null,"target_cut":null,"unresolved":null,"#,
            "\"text\":\"b1 button ? \\u0022Sign in\\u0022 submits F1 post \\u0022/session\\u0022\"}],",
            r#""visibility_note":"hidden attribute only"}}"#,
        );
        assert!(detail.contains(expected), "{detail}");
        assert!(detail.contains(r#""unreadable":0,"#), "{detail}");
    }
}
