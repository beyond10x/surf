//! The stream of `GET /events`, `docs/view-stream.md` "The stream": server-sent events (HTML
//! Standard, event stream format, from memory) with no `Content-Length` and no chunked coding;
//! the body ends when the connection closes.
//!
//! Three threads per stream, each ended by the stream's close:
//!
//! - the connection's own thread writes the queue to the client, one event at a time within
//!   [`Limits::write`](crate::Limits::write), and `: keep-alive` at the heartbeat;
//! - the pump reads the source's [`Reader`] into the queue. The queue holds at most
//!   [`Limits::queue_entries`](crate::Limits::queue_entries) entries and
//!   [`Limits::queue_bytes`](crate::Limits::queue_bytes) bytes, or one frame alone when that
//!   frame is larger, the one being written included; such a frame waits until the queue is
//!   empty. Any other entry past either bound closes this stream alone, and its reader resumes
//!   with `Last-Event-ID`;
//! - the watcher reads the client's side: a client sends nothing after its request, so any byte,
//!   the end of the connection or an error closes the stream at once, and a reader that is gone
//!   frees its stream without waiting for a write to fail.

use std::collections::VecDeque;
use std::io::{ErrorKind, Read};
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use crate::http::{Resume, Shared, Status, StreamSlot, head, later, remaining, write_by};

/// How long one wait of the pump or the writer lasts at most, so that each sees a close or a
/// stop of the server within it.
const POLL: Duration = Duration::from_millis(100);

/// The event name of an [`Item`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    /// `event: entry`, a kept entry.
    Entry,
    /// `event: session`, a session summary; live only, never replayed.
    Session,
}

impl EventKind {
    fn name(self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::Session => "session",
        }
    }
}

/// One item a [`Reader`] yields: `(number, event name, data)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// The `id:` line, the number across all sessions (decision D23).
    pub number: u64,
    /// The `event:` line.
    pub kind: EventKind,
    /// The `data:` line, one JSON line: no CR and no LF.
    pub data: Vec<u8>,
}

/// What [`Reader::next`] gives.
#[derive(Debug)]
pub enum Next {
    /// The next item.
    Item(Item),
    /// Nothing within the wait.
    Idle,
    /// The source ends this reader; the server ends the stream and closes the connection.
    End,
}

/// The answer of [`Source::read_from`](crate::Source::read_from) when an entry above the number
/// is no longer kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TooOld;

/// A reader from a number (decision D23): the kept entries above it in number order, then live.
pub trait Reader: Send {
    /// The next item, waiting at most `wait` for one. A reader that waits longer holds its
    /// stream open past the stream's close.
    fn next(&mut self, wait: Duration) -> Next;
}

/// The queue between the pump and the writer.
#[derive(Default)]
struct Queue {
    frames: VecDeque<Vec<u8>>,
    /// Entries and bytes queued, the frame being written included.
    entries: usize,
    bytes: usize,
    closed: bool,
    ended: bool,
}

struct Line {
    queue: Mutex<Queue>,
    changed: Condvar,
}

impl Line {
    fn lock(&self) -> MutexGuard<'_, Queue> {
        self.queue
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Closes the stream: the writer and the pump stop, the socket is shut both ways, which
    /// wakes a write or a read blocked on it.
    fn close(&self, socket: &TcpStream) {
        self.lock().closed = true;
        self.changed.notify_all();
        let _ = socket.shutdown(Shutdown::Both);
    }
}

/// Where a stream starts.
enum Start {
    /// The replay from the reader's `Last-Event-ID`.
    From(Box<dyn Reader>),
    /// Live after the newest number, with `reset` first when `reset`.
    Newest { reset: bool },
}

/// The stream, after checks 1 to 10 passed: where it starts, the head, then the queue.
pub(crate) fn run(mut stream: TcpStream, shared: &Arc<Shared>, slot: StreamSlot, resume: Resume) {
    let _slot = slot;
    let limits = &shared.config.limits;
    let source = &shared.source;
    let start = match resume {
        Resume::After(number) if number <= source.newest() => match source.read_from(number) {
            Ok(reader) => Start::From(reader),
            Err(TooOld) => Start::Newest { reset: true },
        },
        Resume::Live => Start::Newest { reset: false },
        Resume::After(_) | Resume::Reset => Start::Newest { reset: true },
    };
    // The number of `reset` is the one the reader starts at, read with it.
    let (reader, reset_at) = match start {
        Start::From(reader) => (Some(reader), None),
        Start::Newest { reset } => match source.read_newest() {
            Ok((newest, reader)) => (Some(reader), reset.then_some(newest)),
            // No reader even at the newest number: the head, `reset` when due, and the close
            // after which the reader resumes with `Last-Event-ID`.
            Err(TooOld) => (None, reset.then(|| source.newest())),
        },
    };

    let mut opening = head(Status::Ok, "text/event-stream", None);
    if let Some(newest) = reset_at {
        opening.extend_from_slice(
            format!("id: {newest}\nevent: reset\ndata: {{\"newest\":{newest}}}\n\n").as_bytes(),
        );
    }
    if stream.set_read_timeout(None).is_err()
        || write_by(&mut stream, &opening, later(Instant::now(), limits.write)).is_err()
    {
        return;
    }
    let Some(reader) = reader else {
        let _ = stream.shutdown(Shutdown::Write);
        return;
    };
    let (Ok(pump_socket), Ok(watch_socket)) = (stream.try_clone(), stream.try_clone()) else {
        return;
    };

    let line = Arc::new(Line {
        queue: Mutex::new(Queue::default()),
        changed: Condvar::new(),
    });
    let pump = thread::Builder::new()
        .name("surf-view-pump".to_string())
        .spawn({
            let line = Arc::clone(&line);
            let shared = Arc::clone(shared);
            move || pump(reader, &line, &shared, &pump_socket)
        });
    let watcher = thread::Builder::new()
        .name("surf-view-watch".to_string())
        .spawn({
            let line = Arc::clone(&line);
            move || watch(watch_socket, &line)
        });
    if let (Ok(_), Ok(_)) = (&pump, &watcher) {
        write_queue(&mut stream, &line, shared);
    }
    line.close(&stream);
    if let Ok(pump) = pump {
        let _ = pump.join();
    }
    if let Ok(watcher) = watcher {
        let _ = watcher.join();
    }
}

/// One event's bytes, or `None` when its data would break the framing.
///
/// The data is searched with the slice's own `contains`, which looks for one byte value many
/// bytes at a time; a test build looked at each byte alone, about a second for 100 MiB.
fn frame(item: &Item) -> Option<Vec<u8>> {
    if item.data.contains(&b'\n') || item.data.contains(&b'\r') {
        return None;
    }
    let mut frame =
        format!("id: {}\nevent: {}\ndata: ", item.number, item.kind.name()).into_bytes();
    frame.extend_from_slice(&item.data);
    frame.extend_from_slice(b"\n\n");
    Some(frame)
}

/// Reads the source's reader into the queue until the stream closes, the source ends it, the
/// queue would pass its bound, or an item's data holds a line end.
///
/// A frame alone over [`Limits::queue_bytes`](crate::Limits::queue_bytes), and any frame behind
/// one, waits until the writer has emptied the queue and then goes in alone; the wait ends when
/// the stream closes or the server stops.
fn pump(mut reader: Box<dyn Reader>, line: &Line, shared: &Shared, socket: &TcpStream) {
    let limits = &shared.config.limits;
    loop {
        if line.lock().closed || shared.stopping.load(Ordering::SeqCst) {
            return;
        }
        match reader.next(POLL) {
            Next::Idle => {}
            Next::End => {
                line.lock().ended = true;
                line.changed.notify_all();
                return;
            }
            Next::Item(item) => {
                let Some(frame) = frame(&item) else {
                    return line.close(socket);
                };
                let large = frame.len() > limits.queue_bytes;
                let mut queue = line.lock();
                while (large || queue.bytes > limits.queue_bytes) && queue.entries > 0 {
                    if queue.closed || shared.stopping.load(Ordering::SeqCst) {
                        return;
                    }
                    queue = line
                        .changed
                        .wait_timeout(queue, POLL)
                        .map_or_else(|poisoned| poisoned.into_inner().0, |(queue, _)| queue);
                }
                if queue.closed {
                    return;
                }
                let entries = queue.entries.saturating_add(1);
                let bytes = queue.bytes.saturating_add(frame.len());
                if entries > limits.queue_entries || (bytes > limits.queue_bytes && !large) {
                    drop(queue);
                    return line.close(socket);
                }
                queue.entries = entries;
                queue.bytes = bytes;
                queue.frames.push_back(frame);
                shared.queued_peak.fetch_max(bytes, Ordering::SeqCst);
                drop(queue);
                line.changed.notify_all();
            }
        }
    }
}

/// Closes the stream on anything the client's side does.
fn watch(mut socket: TcpStream, line: &Line) {
    let mut chunk = [0u8; 256];
    loop {
        match socket.read(&mut chunk) {
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::Interrupted | ErrorKind::WouldBlock | ErrorKind::TimedOut
                ) => {}
            _ => return line.close(&socket),
        }
    }
}

enum Job {
    Write(Vec<u8>),
    Beat,
    Stop,
}

/// Writes the queue to the client until the stream closes or ends.
fn write_queue(stream: &mut TcpStream, line: &Line, shared: &Shared) {
    let limits = &shared.config.limits;
    let mut beat = later(Instant::now(), limits.heartbeat);
    loop {
        let job = {
            let mut queue = line.lock();
            loop {
                if queue.closed || shared.stopping.load(Ordering::SeqCst) {
                    break Job::Stop;
                }
                if let Some(frame) = queue.frames.pop_front() {
                    break Job::Write(frame);
                }
                if queue.ended {
                    break Job::Stop;
                }
                let Some(left) = remaining(beat) else {
                    break Job::Beat;
                };
                queue = line
                    .changed
                    .wait_timeout(queue, left.min(POLL))
                    .map_or_else(|poisoned| poisoned.into_inner().0, |(queue, _)| queue);
            }
        };
        let written = match job {
            Job::Stop => return,
            Job::Beat => write_by(
                stream,
                b": keep-alive\n\n",
                later(Instant::now(), limits.write),
            ),
            Job::Write(frame) => {
                let written = write_by(stream, &frame, later(Instant::now(), limits.write));
                let mut queue = line.lock();
                queue.entries = queue.entries.saturating_sub(1);
                queue.bytes = queue.bytes.saturating_sub(frame.len());
                drop(queue);
                // Wakes a pump that waits for an empty queue.
                line.changed.notify_all();
                written
            }
        };
        if written.is_err() {
            return;
        }
        beat = later(Instant::now(), limits.heartbeat);
    }
}
