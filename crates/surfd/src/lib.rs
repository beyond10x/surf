//! `surfd`, the surf daemon: one per user. It owns the list of sessions, starts one session
//! process per session and supervises it, and passes each request of `surf-agent` to the
//! session it names (design section 3 of `docs/design/agent-interface.md`). It holds no page and
//! parses no page, no header and no body; the text of a session it keeps is the address and the
//! title of the status record of the link, quoted ([`link::Quoted`]), and the entries of the
//! link answers in its log ([`log`]): per action its action, its events and its answer, with no
//! token, no body and no password, only their lengths, as the session process wrote them.
//!
//! [`Daemon::start`] is the one start function; the binary `surfd` is a thin `main` over it
//! that reads `XDG_RUNTIME_DIR` and `--idle-exit <seconds>`.
//!
//! # Files
//!
//! Everything is under `$XDG_RUNTIME_DIR/surf/` (mode 0700, owned by the caller), which `surfd`
//! creates when absent and refuses when it is a symbolic link, of another owner or of another
//! mode ([`directory::decide`]): `surfd.lock` (an exclusive lock held for the life of `surfd`),
//! `surfd.sock` (the socket of [`surf_agent::wire`]) and one directory per session,
//! `<identity>/` (mode 0700), holding the session's socket `session.sock`, which the session
//! process binds and only `surfd` connects to ([`link`]). No `XDG_RUNTIME_DIR`: refused, with no
//! fallback to a shared directory. A symbolic link anywhere under `surf/` refuses the start and
//! is never followed or removed.
//!
//! # The peer
//!
//! `surfd` does not read the user id of the peer on `surfd.sock` or on a session socket:
//! `UnixStream::peer_cred` is unstable in rustc 1.99.1 (error E0658, library feature
//! `peer_credentials_unix_socket`), and this crate uses no `unsafe` code and no other crate.
//! What carries that check is the directory, mode 0700 and owned by the caller
//! ([`surf_agent::wire::caller_uid`]), so that only the user's own processes reach the sockets,
//! and the session's token, which the session process checks on every request but `open`.
//! `surfd` passes the token on unchanged and keeps no copy of it.
//!
//! # The session list
//!
//! [`sessions`] says what is listed and when; [`Daemon::sessions`] reads it in process.
//!
//! # The log
//!
//! [`log`] says what is kept of each listed session's entries and how it is bounded;
//! [`Daemon::session_entries`], [`Daemon::entries_after`] and [`Daemon::subscribe_after`] read
//! it in process. It is kept in memory only.
//!
//! # Bounds
//!
//! Every byte from a connection is treated as hostile.
//!
//! - Connections: at most [`CONNECTION_LIMIT`] (64) at once, counted by the accept loop from
//!   the accept to the close, before the connection's thread starts; the 65th is closed
//!   unanswered. One thread per counted connection.
//! - A request: at most [`surf_agent::wire::REQUEST_MAX`] (64 KiB), all of it within
//!   [`surf_agent::wire::REQUEST_TIMEOUT`] (10 seconds) of the accept; a longer or slower one
//!   is closed unanswered. A request that does not decode is answered with status 2.
//! - An identity that is not a UUID in text form is answered with status 2 and touches no path;
//!   a UUID is read in lower case, and every path is built from that lower-case form.
//! - An answer to a client: written within 10 seconds, or the connection is closed.
//! - A session: one request at a time; it answers within [`SESSION_ANSWER_TIMEOUT`] (60
//!   seconds) of the request, waiting for its turn included, or the call ends with status 3 and
//!   `did refused session-timeout`; it reports ready within [`SESSION_READY_TIMEOUT`] (5
//!   seconds) or is killed. A link answer is at most [`link::ANSWER_MAX`] bytes, read into
//!   memory once per call: at most 64 connections of about 12.1 MiB each.
//! - A late answer: after `session-timeout` `surfd` goes on reading the answer for at most
//!   [`LATE_ANSWER_TIMEOUT`] (10 minutes), one thread and at most one link answer per session,
//!   while the session's turn stays taken; the next request to it waits for its turn as above.
//!   The late answer's entries are kept, so none of an action the session recorded is lost
//!   while it lives. A session that does not answer in that time, or breaks the link, is killed
//!   and listed `died`.
//! - The log: per session at most 10,000 entries or 16 MiB, over all sessions at most 64 MiB, a
//!   live reader's queue 1,024 entries ([`log::Bounds`]).
//! - The list: at most [`sessions::ENDED_MAX`] (256) ended sessions. The number of open
//!   sessions is not bounded by `surfd`: each is a process of the user, as any process the user
//!   starts.
//! - Nothing a connection sends reaches an error message or a file: `surfd` writes no log file,
//!   and its own reasons name paths and counts, never a token. What a session's link answer
//!   carries in its entries is kept in memory ([`log`]) and nowhere else.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod daemon;
pub mod directory;
pub mod link;
pub mod log;
pub mod sessions;

pub use daemon::{
    CONNECTION_LIMIT, Config, Daemon, IDLE_EXIT, Identities, KILL_GRACE, LATE_ANSWER_TIMEOUT,
    Refused, SESSION_ANSWER_TIMEOUT, SESSION_PROGRAM, SESSION_READY_TIMEOUT, Started,
};
pub use sessions::{Session, Sessions, State};
