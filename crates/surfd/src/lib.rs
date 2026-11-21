//! `surfd`, the surf daemon: one per user. It owns the list of sessions, starts one session
//! process per session and supervises it, and passes each request of `surf-agent` to the
//! session it names (design section 3 of `docs/design/agent-interface.md`). It holds no page and
//! parses no page, no header and no body; the only text of a session it keeps is the address
//! and the title of the status record of the link, quoted ([`link::Quoted`]).
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
//!   memory once per call: at most 64 connections of about 4.1 MiB each.
//! - The list: at most [`sessions::ENDED_MAX`] (256) ended sessions. The number of open
//!   sessions is not bounded by `surfd`: each is a process of the user, as any process the user
//!   starts.
//! - Nothing a connection sends reaches a log, an error message or a file: `surfd` writes no
//!   log, and its own reasons name paths and counts, never a token.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod daemon;
pub mod directory;
pub mod link;
pub mod sessions;

pub use daemon::{
    CONNECTION_LIMIT, Config, Daemon, IDLE_EXIT, Identities, KILL_GRACE, Refused,
    SESSION_ANSWER_TIMEOUT, SESSION_PROGRAM, SESSION_READY_TIMEOUT, Started,
};
pub use sessions::{Session, Sessions, State};
