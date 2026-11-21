//! The log `surfd` keeps of what its sessions did: per session, the entries each request's link
//! answer carried ([`link::Entry`]), in the order of decision D19 (for each action its action
//! entry, the events of that action, then the answer entry), for the readers of the live view in
//! this process (`epic:live-view`). Nothing of it is written to a file, and it ends with `surfd`.
//!
//! `surfd`, not the session process, stamps each entry it keeps ([`Entry`]): the session, its
//! number within the session from 1, its number across sessions from 1 for each run of `surfd`,
//! and the time it was kept. So a broken session process cannot write into another session or
//! choose a number.
//!
//! # The bounds
//!
//! [`Bounds`], each an option of the `surfd` config ([`crate::Config::log`]); the size of an
//! entry is the bytes of its lines as the link carried them ([`Entry::size`]):
//!
//! - per session at most [`ENTRIES_PER_SESSION`] entries and [`BYTES_PER_SESSION`] bytes: above
//!   either, the session's oldest entries are dropped first and counted
//!   ([`SessionEntries::dropped`]);
//! - over all sessions at most [`BYTES_TOTAL`] bytes: above it, the logs of ended sessions are
//!   dropped whole, the longest-ended first, and such a session's log says so
//!   ([`SessionEntries::log_dropped`]) and counts them dropped; when no ended session's log is
//!   left to drop, the oldest
//!   entries by their number across sessions go next, each counted in its session's `dropped`;
//!   an entry kept for a session whose log was dropped whole is dropped at once: it gets its
//!   numbers and reaches the live readers as every entry does, and is counted, not kept;
//! - a live reader ([`Log::subscribe_after`]) has a queue of [`READER_QUEUE`] entries and is
//!   disconnected when it is full, so a slow reader never holds `surfd` up.
//!
//! A session that `surfd` stops listing (above 256 ended sessions) is removed from the log with
//! its entries ([`Log::remove`]). Every dropped entry, by any of these, makes a reader from a
//! number below it [`OlderThanKept`].

use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError};
use std::time::Duration;

use crate::link::{self, EntryKind};

/// The most entries kept per session: 10,000.
pub const ENTRIES_PER_SESSION: usize = 10_000;

/// The most bytes of entries kept per session: 16 MiB.
pub const BYTES_PER_SESSION: usize = 16 * 1024 * 1024;

/// The most bytes of entries kept over all sessions: 64 MiB.
pub const BYTES_TOTAL: usize = 64 * 1024 * 1024;

/// The queue of a live reader: 1,024 entries.
pub const READER_QUEUE: usize = 1024;

/// The bounds of a [`Log`]. [`Bounds::default`] holds the shipped values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// [`ENTRIES_PER_SESSION`].
    pub entries_per_session: usize,
    /// [`BYTES_PER_SESSION`].
    pub bytes_per_session: usize,
    /// [`BYTES_TOTAL`].
    pub bytes_total: usize,
    /// [`READER_QUEUE`].
    pub reader_queue: usize,
}

impl Default for Bounds {
    fn default() -> Bounds {
        Bounds {
            entries_per_session: ENTRIES_PER_SESSION,
            bytes_per_session: BYTES_PER_SESSION,
            bytes_total: BYTES_TOTAL,
            reader_queue: READER_QUEUE,
        }
    }
}

/// One kept entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Its number across all sessions, from 1 for each run of `surfd`.
    pub number: u64,
    /// The identity of its session.
    pub session: String,
    /// Its number within its session, from 1.
    pub index: u64,
    /// When `surfd` kept it: milliseconds since the Unix epoch.
    pub kept_at: u64,
    /// Its kind.
    pub kind: EntryKind,
    /// Its bytes as the link carried them ([`link::Entry::text`]): quoted fields, kept quoted.
    pub text: String,
}

impl Entry {
    /// The bytes it counts for in the bounds: the bytes of its lines as the link carried them.
    pub fn size(&self) -> usize {
        self.text.len()
    }
}

/// What the log holds of one session ([`Log::session_entries`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionEntries {
    /// The kept entries, in their order.
    pub entries: Vec<Entry>,
    /// How many of its entries were dropped, by any bound: one by one, the oldest first, and
    /// all of them when its log was dropped whole (the example of `docs/view-stream.md` counts
    /// them so).
    pub dropped: u64,
    /// Whether the bound over all sessions dropped its log whole; `entries` is then empty.
    pub log_dropped: bool,
}

/// A reader from a number asked for entries of which one above that number was dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OlderThanKept;

/// A live reader ([`Log::subscribe_after`]): the kept entries above its number, then each entry
/// as it is kept, in order. Disconnected when its queue was full; it then reads what its queue
/// holds and then [`RecvTimeoutError::Disconnected`].
#[derive(Debug)]
pub struct Subscription {
    backlog: VecDeque<Entry>,
    receiver: Receiver<Entry>,
}

impl Subscription {
    /// The next entry, waiting at most `within`.
    pub fn next_within(&mut self, within: Duration) -> Result<Entry, RecvTimeoutError> {
        match self.backlog.pop_front() {
            Some(entry) => Ok(entry),
            None => self.receiver.recv_timeout(within),
        }
    }

    /// The next entry if there is one now.
    pub fn try_next(&mut self) -> Result<Entry, TryRecvError> {
        match self.backlog.pop_front() {
            Some(entry) => Ok(entry),
            None => self.receiver.try_recv(),
        }
    }
}

#[derive(Default)]
struct SessionLog {
    entries: VecDeque<Entry>,
    bytes: usize,
    next_index: u64,
    dropped: u64,
    log_dropped: bool,
}

/// The log of every session `surfd` lists.
pub struct Log {
    bounds: Bounds,
    sessions: HashMap<String, SessionLog>,
    /// The ended sessions, the first ended first.
    ended: VecDeque<String>,
    /// The last number given; 0 before the first entry.
    newest: u64,
    bytes: usize,
    /// The highest number of a dropped entry; 0 while none was dropped.
    dropped_above: u64,
    readers: Vec<SyncSender<Entry>>,
}

impl std::fmt::Debug for Log {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Log")
            .field("bounds", &self.bounds)
            .field("sessions", &self.sessions.len())
            .field("newest", &self.newest)
            .field("bytes", &self.bytes)
            .field("readers", &self.readers.len())
            .finish_non_exhaustive()
    }
}

impl Log {
    /// An empty log within `bounds`.
    pub fn new(bounds: Bounds) -> Log {
        Log {
            bounds,
            sessions: HashMap::new(),
            ended: VecDeque::new(),
            newest: 0,
            bytes: 0,
            dropped_above: 0,
            readers: Vec::new(),
        }
    }

    /// The highest number given so far, 0 before the first entry.
    pub fn newest(&self) -> u64 {
        self.newest
    }

    /// Keep `entry` for `session`, stamped with the next numbers and `kept_at` (milliseconds
    /// since the Unix epoch), hand it to every live reader, then apply the bounds. Its number
    /// across sessions.
    pub fn keep(&mut self, session: &str, entry: link::Entry, kept_at: u64) -> u64 {
        self.newest += 1;
        let number = self.newest;
        let log = self.sessions.entry(session.to_string()).or_default();
        log.next_index += 1;
        let kept = Entry {
            number,
            session: session.to_string(),
            index: log.next_index,
            kept_at,
            kind: entry.kind,
            text: entry.text,
        };
        let size = kept.size();
        self.readers
            .retain(|reader| match reader.try_send(kept.clone()) {
                Ok(()) => true,
                Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => false,
            });
        if log.log_dropped {
            // Its log was dropped whole: an entry that comes after (a `close` kept after the
            // session ended) is dropped at once, so the log stays empty as its detail says.
            log.dropped += 1;
            self.dropped_above = self.dropped_above.max(number);
            return number;
        }
        log.entries.push_back(kept);
        log.bytes += size;
        self.bytes += size;
        self.bound_session(session);
        self.bound_total();
        number
    }

    /// Keep each of `entries` for `session` in order ([`Log::keep`]).
    pub fn keep_all(&mut self, session: &str, entries: Vec<link::Entry>, kept_at: u64) {
        for entry in entries {
            self.keep(session, entry, kept_at);
        }
    }

    /// Note that `session` ended: its log is among the first dropped above the bound over all
    /// sessions, the longest-ended first.
    pub fn end(&mut self, session: &str) {
        if !self.ended.iter().any(|ended| ended == session) {
            self.ended.push_back(session.to_string());
        }
        self.sessions.entry(session.to_string()).or_default();
    }

    /// Remove `session` and its entries: `surfd` no longer lists it.
    pub fn remove(&mut self, session: &str) {
        self.ended.retain(|ended| ended != session);
        if let Some(log) = self.sessions.remove(session) {
            self.bytes -= log.bytes;
            if let Some(last) = log.entries.back() {
                self.dropped_above = self.dropped_above.max(last.number);
            }
        }
    }

    /// The kept entries of `session` in order and its dropped count, or that its log was
    /// dropped; `None` for a session the log does not know.
    pub fn session_entries(&self, session: &str) -> Option<SessionEntries> {
        self.sessions.get(session).map(|log| SessionEntries {
            entries: log.entries.iter().cloned().collect(),
            dropped: log.dropped,
            log_dropped: log.log_dropped,
        })
    }

    /// The newest kept entry of `session` of kind `kind`, read in place, without a copy of the
    /// session's log; `None` when none is kept.
    pub fn newest_of(&self, session: &str, kind: EntryKind) -> Option<&Entry> {
        self.sessions
            .get(session)?
            .entries
            .iter()
            .rev()
            .find(|entry| entry.kind == kind)
    }

    /// Whether the bound over all sessions dropped the log of `session`.
    pub fn log_dropped(&self, session: &str) -> bool {
        self.sessions
            .get(session)
            .is_some_and(|log| log.log_dropped)
    }

    /// The kept entries of every session with a number above `number`, in number order; or
    /// [`OlderThanKept`] when an entry above `number` was dropped.
    pub fn entries_after(&self, number: u64) -> Result<Vec<Entry>, OlderThanKept> {
        if self.dropped_above > number {
            return Err(OlderThanKept);
        }
        let mut after: Vec<Entry> = self
            .sessions
            .values()
            .flat_map(|log| log.entries.iter())
            .filter(|entry| entry.number > number)
            .cloned()
            .collect();
        after.sort_by_key(|entry| entry.number);
        Ok(after)
    }

    /// The entries of [`Log::entries_after`], then every entry as it is kept, through a queue of
    /// [`Bounds::reader_queue`] entries; or [`OlderThanKept`].
    pub fn subscribe_after(&mut self, number: u64) -> Result<Subscription, OlderThanKept> {
        let backlog = self.entries_after(number)?.into();
        let (sender, receiver) = mpsc::sync_channel(self.bounds.reader_queue);
        self.readers.push(sender);
        Ok(Subscription { backlog, receiver })
    }

    /// Drop the oldest entries of `session` while it is above a bound per session.
    fn bound_session(&mut self, session: &str) {
        let Some(log) = self.sessions.get_mut(session) else {
            return;
        };
        while log.entries.len() > self.bounds.entries_per_session
            || log.bytes > self.bounds.bytes_per_session
        {
            let Some(oldest) = log.entries.pop_front() else {
                break;
            };
            log.bytes -= oldest.size();
            log.dropped += 1;
            self.bytes -= oldest.size();
            self.dropped_above = self.dropped_above.max(oldest.number);
        }
    }

    /// While above the bound over all sessions: drop the log of the longest-ended session whose
    /// log is kept, or, when there is none, the oldest entry by its number across sessions.
    fn bound_total(&mut self) {
        while self.bytes > self.bounds.bytes_total {
            let ended = self
                .ended
                .iter()
                .find(|id| self.sessions.get(*id).is_some_and(|log| !log.log_dropped))
                .cloned();
            if let Some(id) = ended {
                if let Some(log) = self.sessions.get_mut(&id) {
                    log.log_dropped = true;
                    if let Some(last) = log.entries.back() {
                        self.dropped_above = self.dropped_above.max(last.number);
                    }
                    log.dropped += log.entries.len() as u64;
                    log.entries.clear();
                    self.bytes -= log.bytes;
                    log.bytes = 0;
                }
                continue;
            }
            let oldest = self
                .sessions
                .iter()
                .filter_map(|(id, log)| log.entries.front().map(|entry| (entry.number, id)))
                .min()
                .map(|(_, id)| id.clone());
            let Some(id) = oldest else {
                break;
            };
            if let Some(log) = self.sessions.get_mut(&id)
                && let Some(entry) = log.entries.pop_front()
            {
                log.bytes -= entry.size();
                log.dropped += 1;
                self.bytes -= entry.size();
                self.dropped_above = self.dropped_above.max(entry.number);
            }
        }
    }
}
