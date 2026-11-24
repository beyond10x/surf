//! The session list: what `surfd` knows about the session processes it started, read in process
//! through [`crate::Daemon::sessions`].
//!
//! It is bookkeeping of the supervisor, not specified rows: the specification holds a session
//! (`surf.agent.Session`) inside the session process. A session is listed from the moment its
//! `open` answer has status 0; an `open` that fails leaves no entry. A listed session is `open`
//! until its process ends; then `closed` when the process ended with status 0 and `surfd` did not
//! kill it (after `close`, the session's own idle timeout, or the end of `surfd`), `died` in
//! every other case. At most [`ENDED_MAX`] ended sessions stay listed; when one more ends, the
//! one that ended first is dropped, its log with it ([`crate::log`]), and an identity that is no
//! longer listed answers `unknown`.
//!
//! The address and the title are kept as the session reported them on the link: quoted by
//! `surf-answer` ([`Quoted`]), printable ASCII. `surfd` does not read them; a reader takes the
//! exact value through [`Quoted::value`], which is `surf_answer::unquote` (decision D26).

use std::time::SystemTime;

use crate::link::Quoted;

/// The most ended sessions kept in the list: 256.
pub const ENDED_MAX: usize = 256;

/// The state of a listed session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The session process runs and answered its `open` with status 0.
    Open,
    /// The session process ended with status 0, not killed by `surfd`.
    Closed,
    /// The session process ended any other way.
    Died,
}

/// One listed session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    /// The identity, a UUID in text form, lower case.
    pub id: String,
    /// The state.
    pub state: State,
    /// When `surfd` started its process for `open`.
    pub opened: SystemTime,
    /// When its last request was answered.
    pub last_action: SystemTime,
    /// When its process ended; `None` while it runs.
    pub ended: Option<SystemTime>,
    /// The process id of its session process.
    pub pid: u32,
    /// The address as the session last reported it, quoted.
    pub address: Option<Quoted>,
    /// The title as the session last reported it, quoted.
    pub title: Option<Quoted>,
    /// Whether the bound over all sessions dropped its log whole ([`crate::log`]).
    pub log_dropped: bool,
    /// The name of the signal its process ended by, such as `SIGXCPU`, when `surfd` did not
    /// kill it; `None` while it runs, after an end with a status, and after `surfd` killed it.
    pub signal: Option<String>,
}

/// The list at one moment, in the order the sessions were opened.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sessions {
    /// The sessions.
    pub list: Vec<Session>,
}

impl Sessions {
    /// The session of identity `id`, if it is listed.
    pub fn get(&self, id: &str) -> Option<&Session> {
        self.list.iter().find(|session| session.id == id)
    }

    /// How many sessions are listed.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether no session is listed.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// The sessions, in the order they were opened.
    pub fn iter(&self) -> impl Iterator<Item = &Session> {
        self.list.iter()
    }
}
