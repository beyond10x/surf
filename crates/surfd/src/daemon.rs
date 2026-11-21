//! [`Daemon::start`], the one start function of `surfd`, and everything it runs: the accept loop,
//! one thread per connection, one supervising thread per session process, and the idle exit.

use std::collections::{HashMap, VecDeque};
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener};
use std::os::unix::fs::FileTypeExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, mpsc};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use surf_agent::wire;
use surf_answer::{Did, Head, RefusalReason, SessionState as HeadState};
use surf_secret::Token;

use crate::directory::{self, Lock};
use crate::link::{self, LinkError, Quoted};
use crate::log::{self, Log, OlderThanKept, SessionEntries, Subscription};
use crate::sessions::{ENDED_MAX, Session, Sessions, State};
use crate::view::{Tables, View};

/// How long a session process has to write `ready`: 5 seconds.
pub const SESSION_READY_TIMEOUT: Duration = Duration::from_secs(5);

/// How long a session has to answer one request: 60 seconds, less than the client's 75.
pub const SESSION_ANSWER_TIMEOUT: Duration = Duration::from_secs(60);

/// How long `surfd` goes on reading an answer that did not come within
/// [`SESSION_ANSWER_TIMEOUT`], with the session's turn taken, before it kills the session: 10
/// minutes. So the entries of a request the session answered late reach the log, in order.
pub const LATE_ANSWER_TIMEOUT: Duration = Duration::from_secs(600);

/// How long a session process has to end after its standard input closed, or after its socket
/// went away, before `surfd` kills it: 2 seconds.
pub const KILL_GRACE: Duration = Duration::from_secs(2);

/// How long `surfd` runs with no open session and no connection: 10 minutes.
pub const IDLE_EXIT: Duration = Duration::from_secs(600);

/// The most connections at once: 64. The 65th is closed unanswered.
pub const CONNECTION_LIMIT: usize = 64;

/// The session program, looked for in the directory of `surfd`'s own executable.
pub const SESSION_PROGRAM: &str = "surf-session";

/// How long `surfd` takes to write one answer to a client.
const ANSWER_WRITE_TIMEOUT: Duration = wire::REQUEST_TIMEOUT;

/// How often a supervising thread asks whether its process ended, once the process's standard
/// output is closed.
const REAP_POLL: Duration = Duration::from_millis(10);

/// How often the loser of the lock tries the socket.
const LOSER_POLL: Duration = Duration::from_millis(10);

/// Where the session identities come from.
#[derive(Clone, Debug, Default)]
pub enum Identities {
    /// Version 4 UUIDs from the operating system random source, through
    /// [`surf_secret::fill_random`].
    #[default]
    Random,
    /// These identities in order, then [`Identities::Random`]. For tests.
    Fixed(Vec<String>),
}

/// What [`Daemon::start`] runs with. [`Config::new`] holds the shipped values.
pub struct Config {
    /// `XDG_RUNTIME_DIR`; `None` when it is unset or empty, which refuses the start.
    pub runtime_dir: Option<PathBuf>,
    /// The session program, an absolute path: [`SESSION_PROGRAM`] beside the running executable.
    pub session_program: PathBuf,
    /// Variables added to a session's environment, which is `surfd`'s own without `SURF_TOKEN`
    /// and `SURF_SESSION`. None in the binary.
    pub session_env: Vec<(OsString, OsString)>,
    /// [`SESSION_READY_TIMEOUT`].
    pub session_ready_timeout: Duration,
    /// [`SESSION_ANSWER_TIMEOUT`].
    pub session_answer_timeout: Duration,
    /// [`LATE_ANSWER_TIMEOUT`].
    pub late_answer_timeout: Duration,
    /// [`KILL_GRACE`].
    pub kill_grace: Duration,
    /// [`IDLE_EXIT`]; `surfd --idle-exit <seconds>` shortens it.
    pub idle_exit: Duration,
    /// [`CONNECTION_LIMIT`].
    pub connection_limit: usize,
    /// The bounds of the log of the sessions' entries: [`log::Bounds::default`].
    pub log: log::Bounds,
    /// [`Identities::Random`].
    pub identities: Identities,
    /// Where the one start line goes, `ready` or `refused <reason>`: standard output in the
    /// binary, nowhere when `None`.
    pub start_line: Option<Box<dyn Write + Send>>,
    /// The limits of the live view's server: [`surf_view::Limits::default`].
    pub view_limits: surf_view::Limits,
    /// Where the view line goes, `http://127.0.0.1:<port>/#<token>` and a line feed, the bytes
    /// of `view-url`: standard error in the binary, nowhere when `None`.
    pub view_line: Option<Box<dyn Write + Send>>,
}

impl Config {
    /// The shipped values for `runtime_dir`.
    pub fn new(runtime_dir: Option<PathBuf>) -> Config {
        let session_program = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join(SESSION_PROGRAM)))
            .unwrap_or_default();
        Config {
            runtime_dir,
            session_program,
            session_env: Vec::new(),
            session_ready_timeout: SESSION_READY_TIMEOUT,
            session_answer_timeout: SESSION_ANSWER_TIMEOUT,
            late_answer_timeout: LATE_ANSWER_TIMEOUT,
            kill_grace: KILL_GRACE,
            idle_exit: IDLE_EXIT,
            connection_limit: CONNECTION_LIMIT,
            log: log::Bounds::default(),
            identities: Identities::Random,
            start_line: None,
            view_limits: surf_view::Limits::default(),
            view_line: None,
        }
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("runtime_dir", &self.runtime_dir)
            .field("session_program", &self.session_program)
            .field("session_env", &self.session_env)
            .field("session_ready_timeout", &self.session_ready_timeout)
            .field("session_answer_timeout", &self.session_answer_timeout)
            .field("late_answer_timeout", &self.late_answer_timeout)
            .field("kill_grace", &self.kill_grace)
            .field("idle_exit", &self.idle_exit)
            .field("connection_limit", &self.connection_limit)
            .field("log", &self.log)
            .field("identities", &self.identities)
            .field("view_limits", &self.view_limits)
            .finish_non_exhaustive()
    }
}

/// Why `surfd` did not start: the reason of its `refused` line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused(pub String);

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Refused {}

/// What [`Daemon::start`] did.
#[derive(Debug)]
pub enum Started {
    /// This process holds the lock and serves `surfd.sock`.
    Daemon(Daemon),
    /// Another `surfd` holds the lock and its socket accepted a connection; this one wrote
    /// `ready` and bound nothing.
    Another,
}

/// A running `surfd`. Dropping it ends it: it stops the live view's server and removes
/// `view-url`, stops accepting, kills its session processes, waits for them and removes
/// `surfd.sock`, and then releases the lock.
pub struct Daemon {
    shared: Arc<Shared>,
    lock: Option<File>,
    /// The live view's server, from the start on.
    view: Option<surf_view::Server>,
}

impl fmt::Debug for Daemon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Daemon")
            .field("socket", &self.shared.socket)
            .finish_non_exhaustive()
    }
}

struct Shared {
    surf: PathBuf,
    socket: PathBuf,
    caller: u32,
    program: PathBuf,
    env: Vec<(OsString, OsString)>,
    ready_timeout: Duration,
    answer_timeout: Duration,
    late_answer_timeout: Duration,
    kill_grace: Duration,
    idle_exit: Duration,
    connection_limit: usize,
    identities: Mutex<VecDeque<String>>,
    table: Mutex<Table>,
    changed: Condvar,
}

/// Everything that changes, under one lock; [`Shared::changed`] is notified on each change.
struct Table {
    entries: HashMap<String, Entry>,
    next_order: u64,
    /// The listed sessions that ended, the first ended first.
    ended: VecDeque<String>,
    connections: usize,
    /// Session processes not yet reaped.
    live: usize,
    idle_since: Option<Instant>,
    stopping: bool,
    /// The entries the listed sessions sent.
    log: Log,
}

impl Table {
    fn note_idle(&mut self) {
        if self.connections > 0 || self.live > 0 {
            self.idle_since = None;
        } else if self.idle_since.is_none() {
            self.idle_since = Some(Instant::now());
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Started, its `open` not yet answered with status 0; not listed.
    Starting,
    Open,
    /// Its last answer said `closed`; it ends.
    Closing,
    Ended(State),
}

struct Entry {
    order: u64,
    listed: bool,
    phase: Phase,
    /// A request is on the link: requests to one session go one at a time.
    turn: bool,
    opened: SystemTime,
    last_action: SystemTime,
    ended: Option<SystemTime>,
    pid: u32,
    address: Option<Quoted>,
    title: Option<Quoted>,
    process: Arc<Process>,
}

struct Process {
    child: Mutex<Child>,
    stdin: Mutex<Option<ChildStdin>>,
    killed: AtomicBool,
}

impl Process {
    /// SIGKILL, unless it was reaped or has ended already; it is then listed `died`.
    fn kill(&self) {
        let mut child = guard(&self.child);
        if let Ok(Some(_)) = child.try_wait() {
            return;
        }
        self.killed.store(true, Ordering::SeqCst);
        let _ = child.kill();
    }

    /// Close its standard input, which ends a session that keeps the rules.
    fn close_input(&self) {
        guard(&self.stdin).take();
    }
}

fn guard<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `Instant::now()` plus `wait`, or as far as an `Instant` reaches.
fn after(wait: Duration) -> Instant {
    let now = Instant::now();
    now.checked_add(wait)
        .or_else(|| now.checked_add(Duration::from_secs(1 << 30)))
        .unwrap_or(now)
}

impl Daemon {
    /// Run `surfd` in this process: check the directory, take the lock, remove what is stale,
    /// bind `$XDG_RUNTIME_DIR/surf/surfd.sock`, and write one line to
    /// [`Config::start_line`]: `ready`, or `refused <reason>` when it returns [`Refused`].
    ///
    /// The loser of the lock binds nothing: it writes `ready` once `surfd.sock` accepts a
    /// connection (which sends nothing and is closed), trying until [`wire::START_TIMEOUT`] from
    /// the call has passed, and returns [`Started::Another`]; it takes the lock itself if it
    /// becomes free in that time.
    pub fn start(mut config: Config) -> Result<Started, Refused> {
        let began = Instant::now();
        let mut line = config.start_line.take();
        let result = begin(config, began);
        let text = match &result {
            Ok(_) => wire::READY.to_string(),
            Err(refused) => format!("{} {}", wire::REFUSED, printable(&refused.0)),
        };
        if let Some(out) = line.as_mut() {
            let _ = out.write_all(format!("{text}\n").as_bytes());
            let _ = out.flush();
        }
        result
    }

    /// The session list now.
    pub fn sessions(&self) -> Sessions {
        snapshot(&guard(&self.shared.table))
    }

    /// Wait at most `within` until `test` holds for the session list; whether it did. Woken on
    /// every change of the list, never by a timer.
    pub fn wait_sessions(&self, within: Duration, test: impl Fn(&Sessions) -> bool) -> bool {
        let deadline = after(within);
        let mut table = guard(&self.shared.table);
        loop {
            if test(&snapshot(&table)) {
                return true;
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            table = self
                .shared
                .changed
                .wait_timeout(table, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }

    /// The kept entries of the listed session `session` in order, its dropped count, or that
    /// its log was dropped ([`crate::log`]); `None` for a session that is not listed.
    pub fn session_entries(&self, session: &str) -> Option<SessionEntries> {
        let table = guard(&self.shared.table);
        if !table.entries.get(session).is_some_and(|entry| entry.listed) {
            return None;
        }
        Some(table.log.session_entries(session).unwrap_or_default())
    }

    /// The kept entries of every session with a number across sessions above `number`, in that
    /// order, or [`OlderThanKept`] when an entry above `number` was dropped.
    pub fn entries_after(&self, number: u64) -> Result<Vec<log::Entry>, OlderThanKept> {
        guard(&self.shared.table).log.entries_after(number)
    }

    /// The entries of [`Daemon::entries_after`], then every new entry as it is kept, in order,
    /// through a queue of [`log::Bounds::reader_queue`] entries; disconnected when the queue is
    /// full. Or [`OlderThanKept`].
    pub fn subscribe_after(&self, number: u64) -> Result<Subscription, OlderThanKept> {
        guard(&self.shared.table).log.subscribe_after(number)
    }

    /// The highest number across sessions given so far, 0 before the first entry.
    pub fn newest_entry(&self) -> u64 {
        guard(&self.shared.table).log.newest()
    }

    /// `$XDG_RUNTIME_DIR/surf/surfd.sock`.
    pub fn socket(&self) -> &Path {
        &self.shared.socket
    }

    /// Block until the idle exit, then end as dropping does.
    pub fn wait(self) {
        let mut table = guard(&self.shared.table);
        while !table.stopping {
            table = self
                .shared
                .changed
                .wait(table)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let shared = &self.shared;
        if let Some(view) = self.view.take() {
            view.stop();
            let _ = directory::remove_view_url(&shared.surf);
        }
        {
            let mut table = guard(&shared.table);
            table.stopping = true;
            shared.changed.notify_all();
        }
        // Wake the accept loop, which sees `stopping` and ends.
        let _ = UnixStream::connect(&shared.socket);
        let processes: Vec<Arc<Process>> = guard(&shared.table)
            .entries
            .values()
            .filter(|entry| !matches!(entry.phase, Phase::Ended(_)))
            .map(|entry| Arc::clone(&entry.process))
            .collect();
        for process in &processes {
            process.close_input();
            process.kill();
        }
        let deadline = after(shared.kill_grace.max(KILL_GRACE));
        let mut table = guard(&shared.table);
        while table.live > 0 {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            table = shared
                .changed
                .wait_timeout(table, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        drop(table);
        if fs::symlink_metadata(&shared.socket).is_ok_and(|m| m.file_type().is_socket()) {
            let _ = fs::remove_file(&shared.socket);
        }
        self.lock.take();
    }
}

fn refused(reason: impl Into<String>) -> Refused {
    Refused(reason.into())
}

fn begin(config: Config, began: Instant) -> Result<Started, Refused> {
    let runtime = config.runtime_dir.ok_or_else(|| {
        refused("XDG_RUNTIME_DIR is not set; surfd does not fall back to a shared directory")
    })?;
    let socket = wire::socket_path(&runtime).map_err(|error| refused(error.to_string()))?;
    if !config.session_program.is_absolute() {
        return Err(refused(
            "the directory of surfd is not known, so surf-session cannot be found",
        ));
    }
    let mut identities = VecDeque::new();
    if let Identities::Fixed(fixed) = &config.identities {
        for id in fixed {
            match wire::session_id(id) {
                Some(lower) if &lower == id => identities.push_back(lower),
                _ => return Err(refused("a fixed identity is not a lower-case UUID")),
            }
        }
    }
    let caller = wire::caller_uid().map_err(|error| {
        refused(format!(
            "the caller's user id cannot be read from /proc/self: {error}"
        ))
    })?;
    let surf = directory::prepare(&runtime, caller).map_err(refused)?;
    let lock_path = surf.join(directory::LOCK);
    let mut lock = directory::lock(&surf, caller).map_err(refused)?;
    let lock = loop {
        match lock {
            Lock::Held(file) => break file,
            Lock::Busy(file) => {
                if UnixStream::connect(&socket).is_ok() {
                    return Ok(Started::Another);
                }
                if began.elapsed() >= wire::START_TIMEOUT {
                    return Err(refused(
                        "another surfd holds the lock and its socket did not answer",
                    ));
                }
                thread::sleep(LOSER_POLL);
                lock = directory::try_lock(file, &lock_path).map_err(refused)?;
            }
        }
    };
    directory::remove_stale(&surf, caller).map_err(refused)?;
    let listener = UnixListener::bind(&socket)
        .map_err(|error| refused(format!("cannot bind {}: {error}", socket.display())))?;
    let view_limits = config.view_limits;
    let view_line = config.view_line;
    let shared = Arc::new(Shared {
        surf,
        socket,
        caller,
        program: config.session_program,
        env: config.session_env,
        ready_timeout: config.session_ready_timeout,
        answer_timeout: config.session_answer_timeout,
        late_answer_timeout: config.late_answer_timeout,
        kill_grace: config.kill_grace,
        idle_exit: config.idle_exit,
        connection_limit: config.connection_limit,
        identities: Mutex::new(identities),
        table: Mutex::new(Table {
            entries: HashMap::new(),
            next_order: 0,
            ended: VecDeque::new(),
            connections: 0,
            live: 0,
            idle_since: Some(Instant::now()),
            stopping: false,
            log: Log::new(config.log),
        }),
        changed: Condvar::new(),
    });
    // From here on a refusal drops `daemon`, which removes surfd.sock and releases the lock.
    let mut daemon = Daemon {
        shared: Arc::clone(&shared),
        lock: Some(lock),
        view: None,
    };
    // The live view's server: after surfd.sock is bound, before `ready` is written.
    daemon.view = Some(start_view(&shared, view_limits, view_line)?);
    let accepting = Arc::clone(&shared);
    thread::Builder::new()
        .name("surfd-accept".into())
        .spawn(move || accept_loop(accepting, listener))
        .map_err(|error| refused(format!("cannot start a thread: {error}")))?;
    let idling = Arc::clone(&shared);
    thread::Builder::new()
        .name("surfd-idle".into())
        .spawn(move || idle_loop(idling))
        .map_err(|error| refused(format!("cannot start a thread: {error}")))?;
    Ok(Started::Daemon(daemon))
}

/// Start the live view's server over `shared`: a listener on 127.0.0.1 port 0, the view token
/// drawn by `surf-secret`, the page's files, then the line `http://127.0.0.1:<port>/#<token>` in
/// `view-url` and on `line`. A failure refuses the start, naming the address or the path, never
/// the token; nothing is left listening, as the server is dropped with the refusal.
fn start_view(
    shared: &Arc<Shared>,
    limits: surf_view::Limits,
    line: Option<Box<dyn Write + Send>>,
) -> Result<surf_view::Server, Refused> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .map_err(|error| refused(format!("cannot listen on 127.0.0.1 for the view: {error}")))?;
    let port = listener
        .local_addr()
        .map_err(|error| refused(format!("cannot read the view's address: {error}")))?
        .port();
    let token =
        Token::draw().map_err(|error| refused(format!("cannot draw the view token: {error}")))?;
    let url = format!("http://127.0.0.1:{port}/#{}\n", token.to_hex());
    let source = View::new(Arc::clone(shared), limits.queue_entries);
    let config = surf_view::Config {
        token,
        host: format!("127.0.0.1:{port}"),
        files: surf_view::page_files(),
        limits,
    };
    let server = surf_view::serve(listener, config, source).map_err(|error| {
        refused(format!(
            "cannot serve the view on 127.0.0.1:{port}: {error}"
        ))
    })?;
    directory::write_view_url(&shared.surf, &url).map_err(refused)?;
    if let Some(mut out) = line {
        let _ = out.write_all(url.as_bytes());
        let _ = out.flush();
    }
    Ok(server)
}

/// What the view reads: the list and the log under one hold of the table lock.
impl Tables for Shared {
    fn hold<R>(&self, read: impl FnOnce(&Sessions, &mut Log) -> R) -> R {
        let mut table = guard(&self.table);
        let sessions = snapshot(&table);
        read(&sessions, &mut table.log)
    }
}

/// `text` with every character outside printable ASCII written as `\x` and two hexadecimal
/// digits per UTF-8 byte, so that the start line is one line.
fn printable(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, ' '..='~') {
            out.push(character);
        } else {
            let mut bytes = [0u8; 4];
            for byte in character.encode_utf8(&mut bytes).bytes() {
                out.push_str(&format!("\\x{byte:02x}"));
            }
        }
    }
    out
}

fn snapshot(table: &Table) -> Sessions {
    let mut listed: Vec<(u64, Session)> = table
        .entries
        .iter()
        .filter(|(_, entry)| entry.listed)
        .map(|(id, entry)| {
            let state = match entry.phase {
                Phase::Ended(state) => state,
                Phase::Starting | Phase::Open | Phase::Closing => State::Open,
            };
            (
                entry.order,
                Session {
                    id: id.clone(),
                    state,
                    opened: entry.opened,
                    last_action: entry.last_action,
                    ended: entry.ended,
                    pid: entry.pid,
                    address: entry.address.clone(),
                    title: entry.title.clone(),
                    log_dropped: table.log.log_dropped(id),
                },
            )
        })
        .collect();
    listed.sort_by_key(|(order, _)| *order);
    Sessions {
        list: listed.into_iter().map(|(_, session)| session).collect(),
    }
}

// --- Connections ----------------------------------------------------------------------------

fn accept_loop(shared: Arc<Shared>, listener: UnixListener) {
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            Err(_) => {
                if guard(&shared.table).stopping {
                    return;
                }
                // Out of descriptors or memory: try again shortly rather than spin.
                thread::sleep(LOSER_POLL);
                continue;
            }
        };
        {
            let mut table = guard(&shared.table);
            if table.stopping {
                return;
            }
            if table.connections >= shared.connection_limit {
                // Closed unanswered when `stream` is dropped.
                continue;
            }
            table.connections += 1;
            table.note_idle();
        }
        let worker = Arc::clone(&shared);
        let spawned = thread::Builder::new()
            .name("surfd-connection".into())
            .spawn(move || {
                let _counted = Counted(Arc::clone(&worker));
                serve(&worker, stream);
            });
        if spawned.is_err() {
            connection_ended(&shared);
        }
    }
}

/// One counted connection, from its accept to its close.
struct Counted(Arc<Shared>);

impl Drop for Counted {
    fn drop(&mut self) {
        connection_ended(&self.0);
    }
}

fn connection_ended(shared: &Shared) {
    let mut table = guard(&shared.table);
    table.connections = table.connections.saturating_sub(1);
    table.note_idle();
    shared.changed.notify_all();
}

fn idle_loop(shared: Arc<Shared>) {
    let mut table = guard(&shared.table);
    loop {
        if table.stopping {
            return;
        }
        let end = table
            .idle_since
            .map(|since| since.checked_add(shared.idle_exit));
        table = match end {
            Some(Some(end)) => {
                let left = end.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    table.stopping = true;
                    shared.changed.notify_all();
                    return;
                }
                shared
                    .changed
                    .wait_timeout(table, left)
                    .unwrap_or_else(PoisonError::into_inner)
                    .0
            }
            Some(None) | None => shared
                .changed
                .wait(table)
                .unwrap_or_else(PoisonError::into_inner),
        };
    }
}

fn serve(shared: &Arc<Shared>, mut stream: UnixStream) {
    let Some(raw) = read_request(&mut stream, after(wire::REQUEST_TIMEOUT)) else {
        return;
    };
    let answer = match wire::Request::decode(&raw) {
        Ok(request) => route(shared, &request, &raw),
        Err(error) => usage(error.to_string()),
    };
    let bytes = answer
        .encode()
        .or_else(|error| unreachable(format!("surfd cannot relay the answer: {error}")).encode());
    if let Ok(bytes) = bytes {
        write_by(&mut stream, &bytes, after(ANSWER_WRITE_TIMEOUT));
    }
}

/// The bytes of one request up to and including its `end` line, at most
/// [`wire::REQUEST_MAX`], all of it by `deadline`; `None` for a connection to close unanswered
/// (longer, ended early, too slow).
fn read_request(stream: &mut UnixStream, deadline: Instant) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut line_start = 0;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        stream.set_read_timeout(Some(left)).ok()?;
        let want = (wire::REQUEST_MAX + 1 - bytes.len()).min(chunk.len());
        let read = match stream.read(&mut chunk[..want]) {
            Ok(0) => return None,
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        };
        let start = bytes.len();
        bytes.extend_from_slice(&chunk[..read]);
        for at in start..bytes.len() {
            if bytes[at] == b'\n' {
                if &bytes[line_start..=at] == b"end\n" {
                    bytes.truncate(at + 1);
                    return Some(bytes);
                }
                line_start = at + 1;
            }
        }
        if bytes.len() > wire::REQUEST_MAX {
            return None;
        }
    }
}

/// Write `bytes`, all of it by `deadline`, or give up.
fn write_by(stream: &mut UnixStream, bytes: &[u8], deadline: Instant) {
    let mut sent = 0;
    while sent < bytes.len() {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() || stream.set_write_timeout(Some(left)).is_err() {
            return;
        }
        match stream.write(&bytes[sent..]) {
            Ok(0) => return,
            Ok(written) => sent += written,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return,
        }
    }
}

fn usage(reason: impl Into<String>) -> wire::Answer {
    wire::Answer {
        status: 2,
        reason: Some(reason.into()),
        text: String::new(),
    }
}

fn unreachable(reason: impl Into<String>) -> wire::Answer {
    wire::Answer {
        status: 3,
        reason: Some(reason.into()),
        text: String::new(),
    }
}

/// An answer `surfd` writes itself: the head line naming `id` (lower case) and `did refused`,
/// with the exit status of the reason table of `docs/agent-answer.md`.
fn own(id: &str, state: HeadState, reason: RefusalReason) -> wire::Answer {
    let mut answer = surf_answer::Answer::new(Head {
        session: id.to_string(),
        state,
    });
    answer.did = Some(Did::Refused(reason));
    let status = match reason {
        RefusalReason::Token | RefusalReason::Closed => 1,
        RefusalReason::Usage => 2,
        RefusalReason::NoSuchSession
        | RefusalReason::SessionDied
        | RefusalReason::SessionTimeout => 3,
    };
    wire::Answer {
        status,
        reason: None,
        text: surf_answer::render(&answer).unwrap_or_default(),
    }
}

fn ended_answer(id: &str, state: State) -> wire::Answer {
    match state {
        State::Closed => own(id, HeadState::Closed, RefusalReason::Closed),
        State::Died | State::Open => own(id, HeadState::Died, RefusalReason::SessionDied),
    }
}

fn relay(answer: link::Answer) -> wire::Answer {
    wire::Answer {
        status: answer.status,
        reason: answer.reason,
        text: answer.text,
    }
}

fn route(shared: &Arc<Shared>, request: &wire::Request, raw: &[u8]) -> wire::Answer {
    if request.command == "open" {
        if request.session.is_some() || request.token.is_some() {
            return usage("open takes no session and no token");
        }
        return open_session(shared, raw);
    }
    let Some(named) = &request.session else {
        return usage(format!("{} needs a session", request.command));
    };
    // An identity that is not a UUID touches no path.
    let Some(id) = wire::session_id(named) else {
        return usage("the session is not a UUID in text form");
    };
    call_session(shared, &id, raw)
}

// --- Sessions -------------------------------------------------------------------------------

/// How a request on the link failed.
enum Failure {
    /// No answer by the deadline; once the request was written, the connection and what was
    /// read of the answer, to read the rest later ([`read_late`]).
    Timeout(Option<Late>),
    /// No session socket, a refused connection, or a connection closed with no byte.
    Gone,
    /// A rule of the link broken.
    Broken,
}

/// The connection of a request whose answer did not come by the call's deadline, and the bytes
/// of the answer read so far.
struct Late {
    stream: UnixStream,
    bytes: Vec<u8>,
}

/// Pass `raw` to the session `id` over one connection to its socket and read its answer.
fn exchange(
    shared: &Shared,
    id: &str,
    raw: &[u8],
    deadline: Instant,
) -> Result<link::Answer, Failure> {
    let path = shared.surf.join(id).join(link::SESSION_SOCKET);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_socket() => {}
        Ok(_) => return Err(Failure::Broken),
        Err(_) => return Err(Failure::Gone),
    }
    let mut stream = UnixStream::connect(&path).map_err(|_| Failure::Gone)?;
    let left = deadline.saturating_duration_since(Instant::now());
    if left.is_zero() {
        return Err(Failure::Timeout(None));
    }
    stream
        .set_write_timeout(Some(left))
        .map_err(|_| Failure::Gone)?;
    if let Err(error) = stream.write_all(raw) {
        return Err(match error.kind() {
            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => Failure::Timeout(None),
            _ => Failure::Gone,
        });
    }
    let mut bytes = Vec::new();
    match link::read_to_end(&mut stream, &mut bytes, deadline) {
        Ok(()) => link::Answer::decode(&bytes).map_err(failure),
        Err(LinkError::Timeout) => Err(Failure::Timeout(Some(Late { stream, bytes }))),
        Err(error) => Err(failure(error)),
    }
}

/// The failure of a link answer that did not decode or was not read whole.
fn failure(error: LinkError) -> Failure {
    match error {
        LinkError::Timeout => Failure::Timeout(None),
        LinkError::Empty | LinkError::Io(_) => Failure::Gone,
        LinkError::TooLarge | LinkError::Malformed(_) => Failure::Broken,
    }
}

/// Release the turn of `id`; for an answer that arrived, note it on the list (last action,
/// address, title, `closed`) and keep its entries while the session is listed.
fn settle(shared: &Shared, id: &str, answer: Option<&mut link::Answer>) {
    let mut table = guard(&shared.table);
    let listed = match table.entries.get_mut(id) {
        Some(entry) => {
            entry.turn = false;
            if let Some(answer) = &answer {
                entry.last_action = SystemTime::now();
                entry.address = answer.address.clone();
                entry.title = answer.title.clone();
                if answer.state == link::SessionState::Closed && entry.phase == Phase::Open {
                    entry.phase = Phase::Closing;
                }
            }
            entry.listed
        }
        None => false,
    };
    if let Some(answer) = answer
        && listed
    {
        table
            .log
            .keep_all(id, std::mem::take(&mut answer.entries), now_millis());
    }
    shared.changed.notify_all();
}

/// Read the rest of an answer that did not come by the call's deadline, for at most
/// [`Config::late_answer_timeout`], while the session's turn stays taken, so that no later
/// request reaches the session before it and its entries are kept in the order the session
/// answered. An answer that does not come in that time, or that breaks the link, kills the
/// session: it is listed `died`, and what it did not send is not kept.
fn read_late(shared: Arc<Shared>, id: String, process: Arc<Process>, mut late: Late) {
    let deadline = after(shared.late_answer_timeout);
    let read = link::read_to_end(&mut late.stream, &mut late.bytes, deadline)
        .and_then(|()| link::Answer::decode(&late.bytes));
    match read {
        Ok(mut answer) => {
            settle(&shared, &id, Some(&mut answer));
            if answer.state == link::SessionState::Closed {
                process.close_input();
            }
        }
        Err(_) => {
            process.kill();
            settle(&shared, &id, None);
        }
    }
}

/// The phase of `id` once it ended, waiting at most `within`; `None` when it still runs, and
/// [`State::Died`] for an entry no longer kept.
fn wait_end(shared: &Shared, id: &str, within: Duration) -> Option<State> {
    let deadline = after(within);
    let mut table = guard(&shared.table);
    loop {
        match table.entries.get(id).map(|entry| entry.phase) {
            Some(Phase::Ended(state)) => return Some(state),
            None => return Some(State::Died),
            Some(_) => {}
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        table = shared
            .changed
            .wait_timeout(table, left)
            .unwrap_or_else(PoisonError::into_inner)
            .0;
    }
}

fn process_of(shared: &Shared, id: &str) -> Option<Arc<Process>> {
    guard(&shared.table)
        .entries
        .get(id)
        .map(|entry| Arc::clone(&entry.process))
}

/// Wait up to the kill grace for the session to end, kill it if it still runs, and answer from
/// its end state.
fn finish(shared: &Shared, id: &str) -> wire::Answer {
    if let Some(state) = wait_end(shared, id, shared.kill_grace) {
        return ended_answer(id, state);
    }
    if let Some(process) = process_of(shared, id) {
        process.kill();
    }
    match wait_end(shared, id, shared.kill_grace.max(KILL_GRACE)) {
        Some(state) => ended_answer(id, state),
        None => own(id, HeadState::Died, RefusalReason::SessionDied),
    }
}

fn call_session(shared: &Arc<Shared>, id: &str, raw: &[u8]) -> wire::Answer {
    let deadline = after(shared.answer_timeout);
    let mut table = guard(&shared.table);
    let process = loop {
        let Some(entry) = table.entries.get_mut(id).filter(|entry| entry.listed) else {
            return own(id, HeadState::Unknown, RefusalReason::NoSuchSession);
        };
        match entry.phase {
            Phase::Ended(state) => return ended_answer(id, state),
            Phase::Closing => {
                drop(table);
                return finish(shared, id);
            }
            Phase::Starting => return own(id, HeadState::Unknown, RefusalReason::NoSuchSession),
            Phase::Open if entry.turn => {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    return own(id, HeadState::Open, RefusalReason::SessionTimeout);
                }
                table = shared
                    .changed
                    .wait_timeout(table, left)
                    .unwrap_or_else(PoisonError::into_inner)
                    .0;
            }
            Phase::Open => {
                entry.turn = true;
                break Arc::clone(&entry.process);
            }
        }
    };
    drop(table);
    let outcome = exchange(shared, id, raw, deadline);
    match outcome {
        Ok(mut answer) => {
            settle(shared, id, Some(&mut answer));
            if answer.state == link::SessionState::Closed {
                process.close_input();
            }
            relay(answer)
        }
        Err(Failure::Timeout(Some(late))) => {
            // The turn stays taken until the late answer is read.
            let reader = {
                let shared = Arc::clone(shared);
                let id = id.to_string();
                let process = Arc::clone(&process);
                thread::Builder::new()
                    .name("surfd-late-answer".into())
                    .spawn(move || read_late(shared, id, process, late))
            };
            if reader.is_err() {
                process.kill();
                settle(shared, id, None);
            }
            own(id, HeadState::Open, RefusalReason::SessionTimeout)
        }
        Err(Failure::Timeout(None)) => {
            settle(shared, id, None);
            own(id, HeadState::Open, RefusalReason::SessionTimeout)
        }
        Err(failure) => {
            settle(shared, id, None);
            match failure {
                Failure::Broken => {
                    process.kill();
                    finish(shared, id)
                }
                _ => finish(shared, id),
            }
        }
    }
}

/// A version 4 UUID in lower case: the next fixed identity, or 16 bytes of
/// [`surf_secret::fill_random`] with the version and variant bits set.
fn next_identity(shared: &Shared) -> io::Result<String> {
    if let Some(id) = guard(&shared.identities).pop_front() {
        return Ok(id);
    }
    let mut bytes = [0u8; 16];
    surf_secret::fill_random(&mut bytes)?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    ))
}

fn forget(shared: &Shared, id: &str) {
    guard(&shared.table).entries.remove(id);
    shared.changed.notify_all();
}

/// Kill a session that did not get listed, wait for its end and drop its entry.
fn abandon(shared: &Shared, id: &str, process: &Process) {
    process.kill();
    let _ = wait_end(shared, id, shared.kill_grace.max(KILL_GRACE));
    forget(shared, id);
}

fn open_session(shared: &Arc<Shared>, raw: &[u8]) -> wire::Answer {
    let id = match next_identity(shared) {
        Ok(id) => id,
        Err(error) => {
            return unreachable(format!("the session identity cannot be drawn: {error}"));
        }
    };
    let dir = shared.surf.join(&id);
    let socket = dir.join(link::SESSION_SOCKET);
    let length = socket.as_os_str().len();
    if length > wire::SOCKET_PATH_MAX {
        return unreachable(format!(
            "the session socket path $XDG_RUNTIME_DIR/{}/<identity>/{} is {length} bytes long; a \
             Unix socket path takes at most {}",
            wire::DIRECTORY,
            link::SESSION_SOCKET,
            wire::SOCKET_PATH_MAX
        ));
    }
    {
        let table = guard(&shared.table);
        if table.stopping {
            return unreachable("surfd is ending");
        }
        if table.entries.contains_key(&id) {
            return unreachable(format!("the session identity {id} is taken"));
        }
    }
    if let Err(reason) = directory::create_session_dir(&dir, shared.caller) {
        return unreachable(reason);
    }
    let mut command = Command::new(&shared.program);
    command
        .arg(&id)
        .arg(&socket)
        .env_remove("SURF_TOKEN")
        .env_remove("SURF_SESSION")
        .envs(shared.env.iter().map(|(name, value)| (name, value)))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .current_dir("/")
        .process_group(0);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let _ = fs::remove_dir(&dir);
            return unreachable(format!(
                "cannot start {}: {error}",
                shared.program.display()
            ));
        }
    };
    let stdout = child.stdout.take();
    let stdin = child.stdin.take();
    let pid = child.id();
    let process = Arc::new(Process {
        child: Mutex::new(child),
        stdin: Mutex::new(stdin),
        killed: AtomicBool::new(false),
    });
    let now = SystemTime::now();
    {
        let mut table = guard(&shared.table);
        let order = table.next_order;
        table.next_order += 1;
        table.entries.insert(
            id.clone(),
            Entry {
                order,
                listed: false,
                phase: Phase::Starting,
                turn: true,
                opened: now,
                last_action: now,
                ended: None,
                pid,
                address: None,
                title: None,
                process: Arc::clone(&process),
            },
        );
        table.live += 1;
        table.note_idle();
    }
    let (ready_sender, ready) = mpsc::channel();
    let supervisor = {
        let shared = Arc::clone(shared);
        let id = id.clone();
        let process = Arc::clone(&process);
        thread::Builder::new()
            .name("surfd-session".into())
            .spawn(move || supervise(shared, id, process, stdout, ready_sender))
    };
    if supervisor.is_err() {
        process.close_input();
        process.kill();
        let _ = guard(&process.child).wait();
        let _ = directory::remove_tree(&dir);
        let mut table = guard(&shared.table);
        table.entries.remove(&id);
        table.live = table.live.saturating_sub(1);
        table.note_idle();
        shared.changed.notify_all();
        return unreachable("cannot start a thread for the session");
    }
    match ready.recv_timeout(shared.ready_timeout) {
        Ok(true) => {}
        Ok(false) => {
            abandon(shared, &id, &process);
            return unreachable("the session ended or wrote another line before it reported ready");
        }
        Err(_) => {
            abandon(shared, &id, &process);
            return unreachable(format!(
                "the session did not report ready within {} seconds",
                shared.ready_timeout.as_secs_f64()
            ));
        }
    }
    match exchange(shared, &id, raw, after(shared.answer_timeout)) {
        Ok(mut answer) if answer.status == 0 => {
            let mut table = guard(&shared.table);
            let mut ended = false;
            if table.entries.contains_key(&id) {
                table
                    .log
                    .keep_all(&id, std::mem::take(&mut answer.entries), now_millis());
            }
            if let Some(entry) = table.entries.get_mut(&id) {
                entry.listed = true;
                entry.turn = false;
                entry.last_action = SystemTime::now();
                entry.address = answer.address.clone();
                entry.title = answer.title.clone();
                match entry.phase {
                    Phase::Ended(_) => ended = true,
                    _ if answer.state == link::SessionState::Closed => {
                        entry.phase = Phase::Closing;
                    }
                    _ => entry.phase = Phase::Open,
                }
            }
            if ended {
                list_ended(&mut table, &id);
            }
            shared.changed.notify_all();
            drop(table);
            relay(answer)
        }
        Ok(answer) => {
            // An open that failed leaves no entry: the session ends, its directory goes.
            process.close_input();
            if wait_end(shared, &id, shared.kill_grace).is_none() {
                process.kill();
                let _ = wait_end(shared, &id, shared.kill_grace.max(KILL_GRACE));
            }
            forget(shared, &id);
            relay(answer)
        }
        Err(failure) => {
            abandon(shared, &id, &process);
            unreachable(match failure {
                Failure::Timeout(_) => format!(
                    "the session did not answer open within {} seconds",
                    shared.answer_timeout.as_secs_f64()
                ),
                Failure::Gone | Failure::Broken => {
                    "the session ended or broke the link while it answered open".to_string()
                }
            })
        }
    }
}

/// Keep `id` among the listed ended sessions, its log among the ended ones, dropping the session
/// that ended first, and its log, above [`ENDED_MAX`].
fn list_ended(table: &mut Table, id: &str) {
    table.ended.push_back(id.to_string());
    table.log.end(id);
    while table.ended.len() > ENDED_MAX {
        if let Some(oldest) = table.ended.pop_front() {
            table.entries.remove(&oldest);
            table.log.remove(&oldest);
        }
    }
}

/// Milliseconds since the Unix epoch, 0 for a clock before it.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// The thread of one session process: read its `ready` line and tell the opener, then hold its
/// standard output (a byte there breaks the link), reap it, remove its directory and set its
/// end state.
fn supervise(
    shared: Arc<Shared>,
    id: String,
    process: Arc<Process>,
    stdout: Option<ChildStdout>,
    ready: mpsc::Sender<bool>,
) {
    match stdout {
        Some(stdout) => {
            let mut reader = BufReader::new(stdout);
            let mut line = Vec::new();
            let read = (&mut reader)
                .take(wire::START_LINE_MAX as u64)
                .read_until(b'\n', &mut line);
            let is_ready = read.is_ok() && line.strip_suffix(b"\n") == Some(link::READY.as_bytes());
            let _ = ready.send(is_ready);
            if is_ready {
                let mut byte = [0u8; 1];
                loop {
                    match reader.read(&mut byte) {
                        Ok(0) => break,
                        Ok(_) => {
                            process.kill();
                            break;
                        }
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                        Err(_) => break,
                    }
                }
            }
        }
        None => {
            let _ = ready.send(false);
        }
    }
    drop(ready);
    let status = loop {
        {
            let mut child = guard(&process.child);
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {}
                Err(_) => break None,
            }
        }
        thread::sleep(REAP_POLL);
    };
    process.close_input();
    let _ = directory::remove_tree(&shared.surf.join(&id));
    let clean =
        status.is_some_and(|status| status.success()) && !process.killed.load(Ordering::SeqCst);
    let state = if clean { State::Closed } else { State::Died };
    let mut table = guard(&shared.table);
    table.live = table.live.saturating_sub(1);
    let mut listed = false;
    if let Some(entry) = table.entries.get_mut(&id) {
        entry.phase = Phase::Ended(state);
        entry.ended = Some(SystemTime::now());
        listed = entry.listed;
    }
    if listed {
        list_ended(&mut table, &id);
    }
    table.note_idle();
    shared.changed.notify_all();
}
