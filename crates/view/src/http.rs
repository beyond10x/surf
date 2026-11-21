//! The server: the listener, one thread per connection, reading and checking a request in the
//! order of `docs/view-stream.md` ("Status codes and their order"), and writing the answer.
//!
//! Every byte of a request is treated as hostile. What bounds each resource:
//!
//! - threads: one per open connection, at most [`Limits::connections`]; past it the accept
//!   thread answers 503 with one non-blocking write and closes, without a thread. A stream adds
//!   two threads (`sse.rs`), at most [`Limits::streams`] streams.
//! - memory per connection: the head buffer, at most [`Limits::head_bytes`]; the header lines
//!   kept, at most [`Limits::head_lines`]; a stream's queue, at most [`Limits::queue_entries`]
//!   and [`Limits::queue_bytes`], or one larger frame alone, whose size the source bounds.
//! - time: the head within [`Limits::receive`] of the accept, or the connection is closed with
//!   no answer; the answer within [`Limits::write`] of its first byte; after the answer at most
//!   [`LINGER`] and [`LINGER_BYTES`] of reading what the client still sends (RFC 9112, tear-down,
//!   from memory), so that the close does not reset the answer away.
//! - panics: no indexing on request bytes outside a bound checked first, numbers parsed with
//!   checked arithmetic, and every lock taken through its poison.

use std::io::{self, ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use surf_secret::Token;

use crate::sse::{self, Reader, TooOld};

/// How long the server reads and discards what a client still sends after its answer.
pub(crate) const LINGER: Duration = Duration::from_secs(1);
/// How many bytes the server reads and discards after an answer, at most.
pub(crate) const LINGER_BYTES: usize = 64 << 10;
/// How long the accept thread waits after a failed `accept`, so that a full file table does not
/// make it spin.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(10);

/// The five security lines of every answer, exactly as `docs/view-stream.md` writes them.
const SECURITY: &str = "Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'\r\n\
X-Content-Type-Options: nosniff\r\n\
Referrer-Policy: no-referrer\r\n\
Cache-Control: no-store\r\n\
Cross-Origin-Resource-Policy: same-origin\r\n";

/// The server's limits, item 8 of `docs/design/live-view.md`. [`Limits::default`] gives the
/// document's values; a test shortens them.
#[derive(Clone, Debug)]
pub struct Limits {
    /// The request line and headers, from the first byte through the empty line: 8,192 bytes.
    pub head_bytes: usize,
    /// Header lines after the request line: 50.
    pub head_lines: usize,
    /// From the accept to the end of the head: 5 s, past which the connection is closed with no
    /// answer.
    pub receive: Duration,
    /// Open connections, the streams included: 16. Past it a new one is answered 503.
    pub connections: usize,
    /// Open streams: 8. Past it `/events` is answered 503.
    pub streams: usize,
    /// Entries queued for one stream: 256. Past it that stream is closed.
    pub queue_entries: usize,
    /// Bytes queued for one stream: 1 MiB, or one frame alone when that frame is larger, which
    /// waits until the queue is empty. Past it that stream is closed.
    pub queue_bytes: usize,
    /// The interval of the `: keep-alive` comment on a stream: 15 s.
    pub heartbeat: Duration,
    /// One answer, or one stream write, from its first byte to its last: 10 s, past which the
    /// connection is closed.
    pub write: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            head_bytes: 8192,
            head_lines: 50,
            receive: Duration::from_secs(5),
            connections: 16,
            streams: 8,
            queue_entries: 256,
            queue_bytes: 1 << 20,
            heartbeat: Duration::from_secs(15),
            write: Duration::from_secs(10),
        }
    }
}

/// One of the page's files. `index.html` answers `/`; every other file answers `/<name>`, and
/// `/index.html` is not a route.
#[derive(Clone, Debug)]
pub struct PageFile {
    /// The file's name, `index.html`, `view.js`: visible ASCII, not starting with `/`.
    pub name: String,
    /// The `Content-Type` of its 200, `text/javascript; charset=utf-8`.
    pub content_type: String,
    /// The file's bytes.
    pub bytes: Vec<u8>,
}

/// What [`serve`] serves.
pub struct Config {
    /// The view token, compared with what a request sends by [`Token::matches`].
    pub token: Token,
    /// The one `Host` value accepted, `127.0.0.1:<port>`, the port the listener's.
    pub host: String,
    /// The page's files.
    pub files: Vec<PageFile>,
    /// The limits.
    pub limits: Limits,
}

/// The data behind the routes, which the caller implements (`surfd`, `story:view-surfd-serves`).
/// The server calls it only for a request that passed every check before the route's own.
pub trait Source: Send + Sync + 'static {
    /// The 200 body of `/api/sessions`.
    fn sessions(&self) -> Vec<u8>;
    /// The 200 body of `/api/sessions/<id>`, or `None` when `id` names no listed session.
    fn session(&self, id: &str) -> Option<Vec<u8>>;
    /// The newest number across all sessions (decision D23), 0 before the first entry.
    fn newest(&self) -> u64;
    /// A reader that yields every kept entry above `number` in number order and then, live,
    /// every new entry and every `session` item; or [`TooOld`] when an entry above `number` is
    /// no longer kept, or when `number` is more than [`Limits::queue_entries`] below the newest
    /// number at the time of the call, so that a replay fits the stream's queue (`surfd`'s
    /// source answers both). `session` items come only live, never from the replay. The newest
    /// number can move between a call of [`Source::newest`] and this one, so `TooOld` is allowed
    /// for a number that [`Source::newest`] gave; a stream from the newest number is opened by
    /// [`Source::read_newest`].
    fn read_from(&self, number: u64) -> Result<Box<dyn Reader>, TooOld>;

    /// The newest number and a reader from it, live only, read together so that no entry kept
    /// meanwhile can make the read [`TooOld`]. The number is the `id:` of a `reset`.
    ///
    /// The default reads [`Source::newest`] and then [`Source::read_from`] that number, and so
    /// can answer `TooOld` when more entries than `read_from` allows are kept between the two
    /// calls; a source that keeps entries while a stream opens gives both in one hold of its
    /// lock (`surfd`'s source does). On `TooOld` the server writes the stream's head, `reset`
    /// when one is due, and closes the stream: its reader resumes with `Last-Event-ID`.
    fn read_newest(&self) -> Result<(u64, Box<dyn Reader>), TooOld> {
        let newest = self.newest();
        self.read_from(newest).map(|reader| (newest, reader))
    }
}

/// What the connection threads share.
pub(crate) struct Shared {
    pub(crate) config: Config,
    pub(crate) source: Box<dyn Source>,
    connections: AtomicUsize,
    pub(crate) streams: AtomicUsize,
    pub(crate) queued_peak: AtomicUsize,
    pub(crate) stopping: AtomicBool,
}

/// A running server. Dropping it, or [`Server::stop`], stops accepting; open streams end
/// within a fraction of a second.
pub struct Server {
    shared: Arc<Shared>,
    address: SocketAddr,
    accept: Option<JoinHandle<()>>,
}

impl Server {
    /// The address the server listens on.
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// The connections open now, streams included.
    pub fn open_connections(&self) -> usize {
        self.shared.connections.load(Ordering::SeqCst)
    }

    /// The streams open now.
    pub fn open_streams(&self) -> usize {
        self.shared.streams.load(Ordering::SeqCst)
    }

    /// The most bytes ever queued for one stream at once, the entry being written included.
    pub fn queued_bytes_peak(&self) -> usize {
        self.shared.queued_peak.load(Ordering::SeqCst)
    }

    /// Stops accepting and waits for the accept thread to end.
    pub fn stop(mut self) {
        self.halt();
    }

    fn halt(&mut self) {
        if let Some(accept) = self.accept.take() {
            self.shared.stopping.store(true, Ordering::SeqCst);
            // Wakes the accept thread, which reads the flag before anything else.
            let _ = TcpStream::connect_timeout(&self.address, Duration::from_secs(1));
            let _ = accept.join();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.halt();
    }
}

/// Serves `config` and `source` on `listener` from a thread of its own.
///
/// Refuses to start, with [`ErrorKind::InvalidInput`], when the listener's address is not
/// 127.0.0.1, when `config.host` is not `127.0.0.1:<the listener's port>`, or when a page
/// file's name or content type is not visible ASCII (a name also not starting with `/`).
pub fn serve<S: Source>(listener: TcpListener, config: Config, source: S) -> io::Result<Server> {
    let address = listener.local_addr()?;
    if address.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) {
        return Err(invalid("the view listens on 127.0.0.1 only"));
    }
    if config.host != format!("127.0.0.1:{}", address.port()) {
        return Err(invalid(
            "the accepted Host is 127.0.0.1 and the listener's port",
        ));
    }
    for file in &config.files {
        let visible = |text: &str| !text.is_empty() && text.bytes().all(is_visible);
        if !visible(&file.name) || file.name.starts_with('/') {
            return Err(invalid(
                "a page file's name is visible ASCII, not starting with /",
            ));
        }
        if !file
            .content_type
            .bytes()
            .all(|b| b == b' ' || is_visible(b))
            || file.content_type.is_empty()
        {
            return Err(invalid("a page file's content type is visible ASCII"));
        }
    }
    listener.set_nonblocking(false)?;
    let shared = Arc::new(Shared {
        config,
        source: Box::new(source),
        connections: AtomicUsize::new(0),
        streams: AtomicUsize::new(0),
        queued_peak: AtomicUsize::new(0),
        stopping: AtomicBool::new(false),
    });
    let accept = thread::Builder::new()
        .name("surf-view-accept".to_string())
        .spawn({
            let shared = Arc::clone(&shared);
            move || accept_loop(&listener, &shared)
        })?;
    Ok(Server {
        shared,
        address,
        accept: Some(accept),
    })
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(ErrorKind::InvalidInput, message)
}

/// Check 1, the connection limit, then a thread per connection.
fn accept_loop(listener: &TcpListener, shared: &Arc<Shared>) {
    loop {
        let accepted = listener.accept();
        if shared.stopping.load(Ordering::SeqCst) {
            return;
        }
        let stream = match accepted {
            Ok((stream, _)) => stream,
            Err(_) => {
                thread::sleep(ACCEPT_BACKOFF);
                continue;
            }
        };
        let accepted_at = Instant::now();
        if shared.connections.load(Ordering::SeqCst) >= shared.config.limits.connections {
            refuse_busy(stream);
            continue;
        }
        let slot = ConnectionSlot::take(shared);
        let spawned = thread::Builder::new()
            .name("surf-view-connection".to_string())
            .spawn({
                let shared = Arc::clone(shared);
                move || {
                    let _slot = slot;
                    handle(stream, &shared, accepted_at);
                }
            });
        // A failed spawn drops the closure, its slot and its stream: the connection closes.
        drop(spawned);
    }
}

/// The 503 past the connection limit: one non-blocking write into the fresh socket's empty send
/// buffer, so that the accept thread never waits on a client; the request is not read.
fn refuse_busy(mut stream: TcpStream) {
    if stream.set_nonblocking(true).is_ok() {
        let answer = refusal(Status::Busy);
        let mut bytes = answer.head;
        bytes.extend_from_slice(&answer.body);
        let _ = stream.write(&bytes);
        let _ = stream.shutdown(Shutdown::Write);
    }
}

/// Counts an open connection for as long as it lives.
struct ConnectionSlot(Arc<Shared>);

impl ConnectionSlot {
    fn take(shared: &Arc<Shared>) -> Self {
        shared.connections.fetch_add(1, Ordering::SeqCst);
        Self(Arc::clone(shared))
    }
}

impl Drop for ConnectionSlot {
    fn drop(&mut self) {
        self.0.connections.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Counts an open stream for as long as it lives; taken only below the stream limit.
pub(crate) struct StreamSlot(Arc<Shared>);

impl StreamSlot {
    fn take(shared: &Arc<Shared>) -> Option<Self> {
        let limit = shared.config.limits.streams;
        shared
            .streams
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |open| {
                (open < limit).then_some(open + 1)
            })
            .ok()
            .map(|_| Self(Arc::clone(shared)))
    }
}

impl Drop for StreamSlot {
    fn drop(&mut self) {
        self.0.streams.fetch_sub(1, Ordering::SeqCst);
    }
}

/// One connection: checks 2 to 10, then the answer or the stream.
fn handle(mut stream: TcpStream, shared: &Arc<Shared>, accepted_at: Instant) {
    let limits = &shared.config.limits;
    let deadline = later(accepted_at, limits.receive);
    let head = match read_head(&mut stream, limits, deadline) {
        Ok(head) => head,
        Err(HeadError::Silent) => return,
        Err(HeadError::TooLarge) => return finish(stream, &refusal(Status::TooLarge), limits),
    };
    match decide(&head, shared) {
        Decision::Answer(answer) => finish(stream, &answer, limits),
        Decision::Stream(slot, resume) => sse::run(stream, shared, slot, resume),
    }
}

/// Writes `answer` within the write limit, then closes in stages: the write side first, then
/// what the client still sends is read and dropped, within [`LINGER`] and [`LINGER_BYTES`].
fn finish(mut stream: TcpStream, answer: &Answer, limits: &Limits) {
    let deadline = later(Instant::now(), limits.write);
    if write_by(&mut stream, &answer.head, deadline).is_err()
        || write_by(&mut stream, &answer.body, deadline).is_err()
    {
        return;
    }
    if stream.shutdown(Shutdown::Write).is_err() {
        return;
    }
    let until = later(Instant::now(), LINGER);
    let mut chunk = [0u8; 1024];
    let mut drained = 0usize;
    while drained < LINGER_BYTES {
        let Some(left) = remaining(until) else { return };
        if stream.set_read_timeout(Some(left)).is_err() {
            return;
        }
        match stream.read(&mut chunk) {
            Ok(0) => return,
            Ok(n) => drained = drained.saturating_add(n),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(_) => return,
        }
    }
}

/// Writes all of `bytes` before `deadline`, or fails.
pub(crate) fn write_by(
    stream: &mut TcpStream,
    mut bytes: &[u8],
    deadline: Instant,
) -> io::Result<()> {
    while !bytes.is_empty() {
        let left = remaining(deadline).ok_or_else(|| io::Error::from(ErrorKind::TimedOut))?;
        stream.set_write_timeout(Some(left))?;
        match stream.write(bytes) {
            Ok(0) => return Err(ErrorKind::WriteZero.into()),
            Ok(n) => bytes = bytes.get(n..).unwrap_or_default(),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// The time left until `deadline`, `None` when none is.
pub(crate) fn remaining(deadline: Instant) -> Option<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
}

/// `from + span`, or a day after `from` when the sum does not fit.
pub(crate) fn later(from: Instant, span: Duration) -> Instant {
    from.checked_add(span)
        .or_else(|| from.checked_add(Duration::from_secs(86_400)))
        .unwrap_or(from)
}

// ---------------------------------------------------------------------------------------------
// Checks 2 and 3: reading the head.

enum HeadError {
    /// Check 2, or the client closed or failed: the connection closes with no answer.
    Silent,
    /// Check 3: 431.
    TooLarge,
}

/// Reads the request line and headers through the first empty line. A line ends at LF here,
/// with or without CR before it, so that a head of bare LF lines ends too and is answered 400 by
/// the syntax check; the size check counts every byte through that empty line.
fn read_head(
    stream: &mut TcpStream,
    limits: &Limits,
    deadline: Instant,
) -> Result<Vec<u8>, HeadError> {
    let mut head = Vec::with_capacity(limits.head_bytes.min(8192));
    let mut chunk = [0u8; 2048];
    let mut line_start = 0usize;
    let mut scanned = 0usize;
    let mut lines = 0usize;
    let most_lines = limits.head_lines.saturating_add(1);
    loop {
        let room = limits.head_bytes.saturating_sub(head.len());
        if room == 0 {
            return Err(HeadError::TooLarge);
        }
        let left = remaining(deadline).ok_or(HeadError::Silent)?;
        stream
            .set_read_timeout(Some(left))
            .map_err(|_| HeadError::Silent)?;
        let want = room.min(chunk.len());
        let n = match stream.read(&mut chunk[..want]) {
            Ok(0) => return Err(HeadError::Silent),
            Ok(n) => n,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return Err(HeadError::Silent),
        };
        head.extend_from_slice(chunk.get(..n).unwrap_or_default());
        while let Some(&byte) = head.get(scanned) {
            if byte == b'\n' {
                let line = head.get(line_start..scanned).unwrap_or_default();
                if line.is_empty() || line == b"\r" {
                    head.truncate(scanned + 1);
                    return Ok(head);
                }
                lines += 1;
                if lines > most_lines {
                    return Err(HeadError::TooLarge);
                }
                line_start = scanned + 1;
            }
            scanned += 1;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Checks 4 to 10.

/// A request that passed the syntax check.
struct Request<'a> {
    method: &'a [u8],
    target: &'a [u8],
    hosts: Vec<&'a [u8]>,
    authorizations: Vec<&'a [u8]>,
    last_event_ids: Vec<&'a [u8]>,
}

/// Check 4: the request line, the line ends, the header lines, the target's form, no body and
/// one `Host`, at most one `Authorization`.
fn parse(head: &[u8]) -> Result<Request<'_>, Status> {
    if head.contains(&0) {
        return Err(Status::BadRequest);
    }
    let body = head.strip_suffix(b"\n").ok_or(Status::BadRequest)?;
    let mut lines = Vec::new();
    for line in body.split(|&b| b == b'\n') {
        let line = line.strip_suffix(b"\r").ok_or(Status::BadRequest)?;
        if line.contains(&b'\r') {
            return Err(Status::BadRequest);
        }
        lines.push(line);
    }
    // The last line is the empty one that ends the head.
    let Some((&empty, lines)) = lines.split_last() else {
        return Err(Status::BadRequest);
    };
    let Some((&request_line, header_lines)) = lines.split_first() else {
        return Err(Status::BadRequest);
    };
    if !empty.is_empty() {
        return Err(Status::BadRequest);
    }

    let mut parts = request_line.split(|&b| b == b' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(Status::BadRequest);
    };
    if method.is_empty() || !method.iter().all(|&b| is_tchar(b)) {
        return Err(Status::BadRequest);
    }
    if target.is_empty() || !target.iter().all(|&b| is_visible(b)) {
        return Err(Status::BadRequest);
    }
    if version != b"HTTP/1.1" {
        return Err(match version {
            [b'H', b'T', b'T', b'P', b'/', major, b'.', minor]
                if major.is_ascii_digit() && minor.is_ascii_digit() =>
            {
                Status::Version
            }
            _ => Status::BadRequest,
        });
    }

    let mut request = Request {
        method,
        target,
        hosts: Vec::new(),
        authorizations: Vec::new(),
        last_event_ids: Vec::new(),
    };
    let mut framing = false;
    for &line in header_lines {
        if matches!(line.first(), Some(b' ' | b'\t')) {
            return Err(Status::BadRequest);
        }
        let colon = line
            .iter()
            .position(|&b| b == b':')
            .ok_or(Status::BadRequest)?;
        let (name, rest) = line.split_at(colon);
        if name.is_empty() || !name.iter().all(|&b| is_tchar(b)) {
            return Err(Status::BadRequest);
        }
        let value = trim(rest.get(1..).unwrap_or_default());
        if name.eq_ignore_ascii_case(b"host") {
            request.hosts.push(value);
        } else if name.eq_ignore_ascii_case(b"authorization") {
            request.authorizations.push(value);
        } else if name.eq_ignore_ascii_case(b"last-event-id") {
            request.last_event_ids.push(value);
        } else if name.eq_ignore_ascii_case(b"content-length")
            || name.eq_ignore_ascii_case(b"transfer-encoding")
        {
            framing = true;
        }
    }
    if request.target.first() != Some(&b'/')
        || framing
        || request.hosts.len() != 1
        || request.authorizations.len() > 1
    {
        return Err(Status::BadRequest);
    }
    Ok(request)
}

/// A field value without the optional whitespace around it.
fn trim(value: &[u8]) -> &[u8] {
    let start = value.iter().position(|&b| b != b' ' && b != b'\t');
    let end = value.iter().rposition(|&b| b != b' ' && b != b'\t');
    match (start, end) {
        (Some(start), Some(end)) => value.get(start..=end).unwrap_or_default(),
        _ => &[],
    }
}

/// A `tchar` of RFC 9110, the bytes of a method and a field name.
fn is_tchar(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}

/// Visible ASCII, `%x21-7E`.
fn is_visible(b: u8) -> bool {
    (0x21..=0x7e).contains(&b)
}

enum Route<'a> {
    File(&'a PageFile),
    Sessions,
    Session(&'a str),
    Events,
}

/// Check 7: the target, byte for byte. The API routes are matched before the page files, so a
/// page file can never stand in for a route that needs the token.
fn route<'a>(target: &'a [u8], files: &'a [PageFile]) -> Option<Route<'a>> {
    let target = std::str::from_utf8(target).ok()?;
    match target {
        "/api/sessions" => return Some(Route::Sessions),
        "/events" => return Some(Route::Events),
        _ => {}
    }
    if let Some(id) = target.strip_prefix("/api/sessions/") {
        return is_identity(id).then_some(Route::Session(id));
    }
    let name = target.strip_prefix('/')?;
    let name = if name.is_empty() {
        "index.html"
    } else if name == "index.html" {
        return None;
    } else {
        name
    };
    files.iter().find(|file| file.name == name).map(Route::File)
}

/// A session identity in its text form: lower-case hexadecimal digits in groups of 8, 4, 4, 4
/// and 12 joined by `-`.
fn is_identity(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(at, b)| match at {
            8 | 13 | 18 | 23 => b == b'-',
            _ => b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
        })
}

/// Check 8: exactly `Bearer <64 lower-case hexadecimal digits>`, compared by `surf-secret` in
/// constant time. Parsing the digits stops at the first that is not one, which tells a client
/// only what it sent; the comparison with the view token reads every byte.
fn authorized(authorization: Option<&[u8]>, token: &Token) -> bool {
    let Some(digits) = authorization.and_then(|value| value.strip_prefix(b"Bearer ")) else {
        return false;
    };
    let Ok(digits) = std::str::from_utf8(digits) else {
        return false;
    };
    Token::from_hex(digits).is_some_and(|sent| token.matches(&sent))
}

enum Decision {
    Answer(Answer),
    Stream(StreamSlot, Resume),
}

/// Where a stream starts, from its `Last-Event-ID`.
pub(crate) enum Resume {
    /// No `Last-Event-ID`: live after the newest number.
    Live,
    /// `Last-Event-ID: <n>`, decimal digits of at most 20 that fit a `u64`.
    After(u64),
    /// Any other `Last-Event-ID`, or more than one: `reset`.
    Reset,
}

/// Checks 4 to 10, the first that fails answering.
fn decide(head: &[u8], shared: &Arc<Shared>) -> Decision {
    let config = &shared.config;
    let request = match parse(head) {
        Ok(request) => request,
        Err(status) => return Decision::Answer(refusal(status)),
    };
    if request.method != b"GET" {
        return Decision::Answer(refusal(Status::MethodNotAllowed));
    }
    if request.hosts.first().copied() != Some(config.host.as_bytes()) {
        return Decision::Answer(refusal(Status::Forbidden));
    }
    let Some(route) = route(request.target, &config.files) else {
        return Decision::Answer(refusal(Status::NotFound));
    };
    if let Route::File(file) = route {
        return Decision::Answer(answer(Status::Ok, &file.content_type, file.bytes.clone()));
    }
    if !authorized(request.authorizations.first().copied(), &config.token) {
        return Decision::Answer(refusal(Status::Unauthorized));
    }
    match route {
        Route::File(_) => Decision::Answer(refusal(Status::NotFound)),
        Route::Sessions => Decision::Answer(answer(
            Status::Ok,
            "application/json",
            shared.source.sessions(),
        )),
        Route::Session(id) => match shared.source.session(id) {
            Some(body) => Decision::Answer(answer(Status::Ok, "application/json", body)),
            None => Decision::Answer(refusal(Status::NotFound)),
        },
        Route::Events => match StreamSlot::take(shared) {
            Some(slot) => Decision::Stream(slot, resume(&request.last_event_ids)),
            None => Decision::Answer(refusal(Status::Busy)),
        },
    }
}

fn resume(values: &[&[u8]]) -> Resume {
    match values {
        [] => Resume::Live,
        [digits] if (1..=20).contains(&digits.len()) && digits.iter().all(u8::is_ascii_digit) => {
            digits
                .iter()
                .try_fold(0u64, |number, &digit| {
                    number.checked_mul(10)?.checked_add(u64::from(digit - b'0'))
                })
                .map_or(Resume::Reset, Resume::After)
        }
        _ => Resume::Reset,
    }
}

// ---------------------------------------------------------------------------------------------
// The answer.

/// The status codes of `docs/view-stream.md`, with their reasons and refusal words.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Ok,
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    TooLarge,
    Busy,
    Version,
}

impl Status {
    fn line(self) -> &'static str {
        match self {
            Self::Ok => "HTTP/1.1 200 OK\r\n",
            Self::BadRequest => "HTTP/1.1 400 Bad Request\r\n",
            Self::Unauthorized => "HTTP/1.1 401 Unauthorized\r\n",
            Self::Forbidden => "HTTP/1.1 403 Forbidden\r\n",
            Self::NotFound => "HTTP/1.1 404 Not Found\r\n",
            Self::MethodNotAllowed => "HTTP/1.1 405 Method Not Allowed\r\n",
            Self::TooLarge => "HTTP/1.1 431 Request Header Fields Too Large\r\n",
            Self::Busy => "HTTP/1.1 503 Service Unavailable\r\n",
            Self::Version => "HTTP/1.1 505 HTTP Version Not Supported\r\n",
        }
    }

    fn word(self) -> &'static str {
        match self {
            Self::Ok => "",
            Self::BadRequest => "bad-request",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "host",
            Self::NotFound => "not-found",
            Self::MethodNotAllowed => "method",
            Self::TooLarge => "too-large",
            Self::Busy => "busy",
            Self::Version => "version",
        }
    }
}

pub(crate) struct Answer {
    pub(crate) head: Vec<u8>,
    pub(crate) body: Vec<u8>,
}

fn answer(status: Status, content_type: &str, body: Vec<u8>) -> Answer {
    Answer {
        head: head(status, content_type, Some(body.len())),
        body,
    }
}

fn refusal(status: Status) -> Answer {
    let body = format!("{{\"error\":\"{}\"}}", status.word()).into_bytes();
    answer(status, "application/json", body)
}

/// The head of an answer, its lines in the document's order; `length` is `None` on the stream
/// only.
pub(crate) fn head(status: Status, content_type: &str, length: Option<usize>) -> Vec<u8> {
    let mut text = String::with_capacity(512);
    text.push_str(status.line());
    text.push_str("Content-Type: ");
    text.push_str(content_type);
    text.push_str("\r\n");
    if let Some(length) = length {
        text.push_str(&format!("Content-Length: {length}\r\n"));
    }
    if status == Status::MethodNotAllowed {
        text.push_str("Allow: GET\r\n");
    }
    if status == Status::Unauthorized {
        text.push_str("WWW-Authenticate: Bearer\r\n");
    }
    text.push_str(SECURITY);
    text.push_str("Date: ");
    text.push_str(&imf_fixdate(SystemTime::now()));
    text.push_str("\r\nConnection: close\r\n\r\n");
    text.into_bytes()
}

/// `time` as an IMF-fixdate of RFC 9110, `Sat, 21 Nov 2026 02:00:00 GMT`; a time before the
/// epoch is written as the epoch.
fn imf_fixdate(time: SystemTime) -> String {
    const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let seconds = time
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let days = seconds / 86_400;
    let of_day = seconds % 86_400;
    // The proleptic Gregorian date from days since 1970-01-01, counted in 400-year eras that
    // start on 1 March.
    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let day_of_era = shifted % 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
        DAYS[(days % 7) as usize],
        day,
        MONTHS[(month - 1) as usize],
        year,
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60,
    )
}
