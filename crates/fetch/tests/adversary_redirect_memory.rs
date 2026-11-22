//! Adversary case for story `agent-fetch-redirects`, and the bounds of memory the crate
//! documentation of `surf-fetch` states ("Memory"): a server the agent was sent to redirects a
//! short asked address to one of about 64 KiB, then serves a body at the body limit; and a
//! redirect whose `Location` takes the whole header limit, resolved with a small body limit. And
//! case 14 of the story that sends a `POST` (wave 9): a request body of 1 MiB is not copied; and
//! the peak while a request body of 4 MiB is written twice.
//!
//! Why this file holds `unsafe` (as does `crates/html/tests/common/mod.rs`): what a fetch holds is
//! only seen by counting what it allocates, and a counting allocator is a `GlobalAlloc`, an
//! `unsafe` trait whose methods are `unsafe fn`. The allocator passes every call on to the
//! system allocator unchanged and only counts sizes. No file under any `src/` holds `unsafe`;
//! every hand-written crate forbids it.
//!
//! The allocator is the global allocator of the one test binary of `surf-fetch`, and it counts
//! for each thread: a case reads what the thread that calls the fetch allocated. The fetches of
//! these cases go to `127.0.0.1`, an IP address, so the fetch resolves no name on a thread of its
//! own and allocates only on the calling thread; the test's server runs on another thread and is
//! not counted. So a case counts only its own fetch, also when `cargo test` runs the tests of the
//! binary side by side in one process.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use surf_fetch::{fetch, send, Failure, Limits, Method, Request, Response, MAX_ADDRESS_BYTES};

struct Counting;

thread_local! {
    /// The bytes allocated and not yet freed by this thread.
    static LIVE: Cell<isize> = const { Cell::new(0) };
    /// The most of [`LIVE`] since the last [`held_while`] began.
    static PEAK: Cell<isize> = const { Cell::new(0) };
}

fn grow(n: usize) {
    let _ = LIVE.try_with(|live| {
        let now = live.get() + n as isize;
        live.set(now);
        let _ = PEAK.try_with(|peak| peak.set(peak.get().max(now)));
    });
}

fn shrink(n: usize) {
    let _ = LIVE.try_with(|live| live.set(live.get() - n as isize));
}

// SAFETY: every call is passed on to the system allocator unchanged; only sizes are counted.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            grow(layout.size());
        }
        p
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            grow(layout.size());
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        shrink(layout.size());
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            if new_size >= layout.size() {
                grow(new_size - layout.size());
            } else {
                shrink(layout.size() - new_size);
            }
        }
        p
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// The value of `f` and the most bytes this thread held while `f` ran, beyond what it held
/// before.
fn held_while<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = LIVE.with(Cell::get);
    PEAK.with(|peak| peak.set(before));
    let out = f();
    let held = (PEAK.with(Cell::get) - before).max(0) as usize;
    (out, held)
}

/// The bytes one read from the connection asks for: `READ_CHUNK_BYTES` of the crate, 8 KiB.
const READ_BUFFER: usize = 8 * 1024;

/// The longest chunk-size line: `MAX_CHUNK_LINE_BYTES` of the crate, 1 KiB.
const CHUNK_LINE: usize = 1024;

/// Read one request head with a buffer on the stack, so the server allocates nothing while the
/// fetch is measured. Returns the bytes of the head.
fn read_head(stream: &mut TcpStream) -> usize {
    let mut buf = [0u8; 4096];
    let mut last = [0u8; 4];
    let mut total = 0;
    loop {
        let Ok(n) = stream.read(&mut buf) else {
            return total;
        };
        if n == 0 {
            return total;
        }
        for &b in &buf[..n] {
            total += 1;
            last = [last[1], last[2], last[3], b];
            if &last == b"\r\n\r\n" {
                return total;
            }
        }
    }
}

/// A server on `127.0.0.1` port 0 that answers its connections with `replies` in order, one
/// each, and then ends. It allocates nothing once it accepts, and frees no reply: the caller
/// holds them until it has measured.
fn serve(replies: &[Arc<Vec<u8>>]) -> (u16, JoinHandle<()>) {
    let replies: Vec<Arc<Vec<u8>>> = replies.to_vec();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local address").port();
    let server = thread::spawn(move || {
        for reply in &replies {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
            read_head(&mut stream);
            let _ = stream.write_all(reply);
            let _ = stream.shutdown(Shutdown::Write);
            let mut sink = [0u8; 1024];
            while let Ok(n) = stream.read(&mut sink) {
                if n == 0 {
                    break;
                }
            }
        }
    });
    (port, server)
}

/// The fetch of `asked` and the most bytes it held at once beyond what was held before it.
fn measure(asked: &str, limits: &Limits) -> (Result<Response, Failure>, usize) {
    held_while(|| fetch(asked, limits))
}

/// A 302 whose `Location` is `location`.
fn redirect(location: &str) -> Vec<u8> {
    format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\n\r\n").into_bytes()
}

#[test]
fn adversary_a_redirect_to_a_64_kib_address_holds_at_most_the_documented_sum_while_the_body_is_read(
) {
    let limits = Limits::default();
    // The Location fits the header limit; the address it resolves to is at most 64 KiB.
    let location = format!("/{}", "p".repeat(65_000));
    let redirect: Arc<Vec<u8>> = Arc::new(redirect(&location));
    assert!(redirect.len() <= limits.max_header_bytes);
    // The page's head takes the whole header limit with its media type, the one thing of the
    // head kept while the body is read.
    let head = |media_type: &str| {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {media_type}\r\nContent-Length: {}\r\n\r\n",
            limits.max_body_bytes
        )
    };
    let media_type = format!(
        "text/{}",
        "x".repeat(limits.max_header_bytes - head("text/").len())
    );
    let mut page = head(&media_type).into_bytes();
    assert_eq!(page.len(), limits.max_header_bytes);
    page.resize(page.len() + limits.max_body_bytes, b'x');
    let page: Arc<Vec<u8>> = Arc::new(page);
    let replies = [redirect, page];
    let (port, server) = serve(&replies);
    let prefix = format!("http://127.0.0.1:{port}");
    let asked = format!("{prefix}/a");
    let expected_url = format!("{prefix}{location}");

    let (result, held) = measure(&asked, &limits);

    server.join().expect("server thread");
    let response = result.expect("the redirect is followed and the page is a response");
    assert_eq!(response.url, expected_url);
    assert_eq!(response.media_type, media_type);
    assert_eq!(response.body.len(), limits.max_body_bytes);
    // The crate documentation, "Memory", while the body is read: the header limit, the body
    // limit and one byte, one read buffer, one chunk-size line, and the response URL.
    let bound = limits.max_header_bytes
        + limits.max_body_bytes
        + 1
        + READ_BUFFER
        + CHUNK_LINE
        + MAX_ADDRESS_BYTES;
    assert_eq!(bound, 65_536 + 16_777_216 + 1 + 8_192 + 1_024 + 65_536);
    assert_eq!(bound, 16_917_505);
    assert!(
        held <= bound,
        "crate documentation of surf-fetch, \"Memory\": at the default limits a fetch redirected \
         to an address of {} bytes held {held} bytes while the body was read, {} bytes over the \
         header limit plus the body limit and one byte plus one read buffer plus one chunk-size \
         line plus MAX_ADDRESS_BYTES ({bound})",
        response.url.len(),
        held.saturating_sub(bound)
    );
}

/// The crate documentation, "Memory", while a `Location` is resolved: a `Location` at the
/// header limit, of the costliest shape found (a path of empty segments), with a body limit of
/// 16 bytes, so no moment of a body comes near the moment of resolving. Once against the short
/// address asked, once against a URL that is itself the 64 KiB result of such a `Location`.
#[test]
fn resolving_a_location_at_the_header_limit_holds_at_most_30_or_54_times_the_header_limit() {
    let limits = Limits {
        max_body_bytes: 16,
        ..Limits::default()
    };
    assert_eq!(limits.max_header_bytes, 65_536);
    let fixed = redirect("").len();
    let location = format!("/a{}", "/".repeat(limits.max_header_bytes - fixed - 2));
    let redirect = Arc::new(redirect(&location));
    assert_eq!(redirect.len(), limits.max_header_bytes);
    let page = Arc::new(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok".to_vec());

    for (redirects, multiple, bound) in [(1, 30, 1_966_080), (2, 54, 3_538_944)] {
        assert_eq!(bound, multiple * limits.max_header_bytes);
        let mut replies = vec![Arc::clone(&redirect); redirects];
        replies.push(Arc::clone(&page));
        let (port, server) = serve(&replies);
        let prefix = format!("http://127.0.0.1:{port}");
        let asked = format!("{prefix}/a");

        let (result, held) = measure(&asked, &limits);

        server.join().expect("server thread");
        let response = result.expect("the redirects are followed and the page is a response");
        assert_eq!(response.url, format!("{prefix}{location}"));
        assert!(response.url.len() <= MAX_ADDRESS_BYTES);
        assert_eq!(response.body, b"ok");
        assert!(
            held <= bound,
            "crate documentation of surf-fetch, \"Memory\": resolving a Location of {} bytes \
             after {} redirect(s) held {held} bytes, more than {multiple} times the header \
             limit ({bound})",
            location.len(),
            redirects - 1
        );
    }
}

/// The longest request head [`serve_posts`] keeps, on the stack.
const POST_HEAD: usize = 1024;

/// The value of `Content-Length` in `head`, read without allocating; 0 when it has none.
fn declared_length(head: &[u8]) -> usize {
    const NAME: &[u8] = b"\r\nContent-Length: ";
    let Some(at) = head.windows(NAME.len()).position(|w| w == NAME) else {
        return 0;
    };
    head[at + NAME.len()..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .fold(0, |n, &b| n * 10 + usize::from(b - b'0'))
}

/// Read one request head byte by byte into `head` on the stack, then the `Content-Length` bytes
/// of its body with a buffer on the stack, so the server allocates nothing while the fetch is
/// measured. Returns the bytes of the body read.
fn read_post(stream: &mut TcpStream) -> usize {
    let mut head = [0u8; POST_HEAD];
    let mut len = 0;
    let mut byte = [0u8; 1];
    while len < POST_HEAD && !head[..len].ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(1) => {
                head[len] = byte[0];
                len += 1;
            }
            _ => return 0,
        }
    }
    let mut left = declared_length(&head[..len]);
    let mut buf = [0u8; 8192];
    let mut read = 0;
    while left > 0 {
        let want = left.min(buf.len());
        match stream.read(&mut buf[..want]) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                left -= n;
                read += n;
            }
        }
    }
    read
}

/// [`serve`] for requests with a body: each request's body is read by its `Content-Length`
/// before the reply is written. Returns the bytes of body each request carried.
fn serve_posts(replies: &[Arc<Vec<u8>>]) -> (u16, JoinHandle<Vec<usize>>) {
    let replies: Vec<Arc<Vec<u8>>> = replies.to_vec();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local address").port();
    let server = thread::spawn(move || {
        let mut bodies = Vec::with_capacity(replies.len());
        for reply in &replies {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
            bodies.push(read_post(&mut stream));
            let _ = stream.write_all(reply);
            let _ = stream.shutdown(Shutdown::Write);
            let mut sink = [0u8; 1024];
            while let Ok(n) = stream.read(&mut sink) {
                if n == 0 {
                    break;
                }
            }
        }
        bodies
    });
    (port, server)
}

/// Case 14 of the story that sends a `POST`: the request body is the caller's and is not copied. A
/// `Post` of 1 MiB goes through one 307, which repeats it, and the page then has a body at the
/// default limit: the fetch holds at most the documented sum while the body is read, the same
/// as a `Get`, though the request body is 1 MiB.
#[test]
fn a_post_body_is_not_copied() {
    let limits = Limits::default();
    let body = vec![b'b'; 1024 * 1024];
    let redirect: Arc<Vec<u8>> = Arc::new(
        b"HTTP/1.1 307 Temporary Redirect\r\nLocation: /b\r\nContent-Length: 0\r\n\r\n".to_vec(),
    );
    let mut page = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
        limits.max_body_bytes
    )
    .into_bytes();
    page.resize(page.len() + limits.max_body_bytes, b'x');
    let page: Arc<Vec<u8>> = Arc::new(page);
    let (port, server) = serve_posts(&[redirect, page]);
    let asked = format!("http://127.0.0.1:{port}/a");
    let request = Request {
        address: &asked,
        method: Method::Post,
        body: Some(&body),
        media_type: Some("application/octet-stream"),
    };

    let (result, held) = held_while(|| send(&request, &limits));

    let bodies = server.join().expect("server thread");
    assert_eq!(bodies, [body.len(), body.len()], "the 307 repeats the body");
    let response = result.expect("the redirect is followed and the page is a response");
    assert_eq!(response.url, format!("http://127.0.0.1:{port}/b"));
    assert_eq!(response.body.len(), limits.max_body_bytes);
    let bound = 16_917_505;
    assert!(
        held <= bound,
        "crate documentation of surf-fetch, \"Memory\": a Post of {} bytes through a 307 held \
         {held} bytes while the body at the limit was read, {} bytes over {bound}",
        body.len(),
        held.saturating_sub(bound)
    );
}

/// The peak while a request body is written, after the adversary's pass on the story that sends
/// a `POST` (wave 9). A `Post` of 4 MiB goes through one 308, which repeats it, to a page of 2
/// bytes, so no response body hides a copy made and dropped while the request is written: the
/// peak over the whole fetch is that of the fetch without its body.
///
/// The bound, from the crate's limits: the addresses and the response heads are short, so the
/// largest thing a fetch allocates is the buffer of a head being read, at most one read buffer
/// (a head shorter than one read takes no more); everything else (the request head, the address
/// parsed, the response URL, the fields of a head) is shorter than the header limit. So the
/// header limit plus one read buffer, 73,728 bytes at the defaults, less than a 56th of the body:
/// one copy of it is red.
#[test]
fn a_post_body_written_twice_is_not_copied_while_it_is_written() {
    let limits = Limits::default();
    let body = vec![b'b'; 4 * 1024 * 1024];
    let redirect: Arc<Vec<u8>> = Arc::new(
        b"HTTP/1.1 308 Permanent Redirect\r\nLocation: /b\r\nContent-Length: 0\r\n\r\n".to_vec(),
    );
    let page: Arc<Vec<u8>> = Arc::new(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nok".to_vec(),
    );
    let (port, server) = serve_posts(&[redirect, page]);
    let asked = format!("http://127.0.0.1:{port}/a");
    let request = Request {
        address: &asked,
        method: Method::Post,
        body: Some(&body),
        media_type: Some("application/octet-stream"),
    };

    let (result, held) = held_while(|| send(&request, &limits));

    let bodies = server.join().expect("server thread");
    assert_eq!(bodies, [body.len(), body.len()], "the 308 repeats the body");
    let response = result.expect("the redirect is followed and the page is a response");
    assert_eq!(response.url, format!("http://127.0.0.1:{port}/b"));
    assert_eq!(response.body, b"ok");
    let bound = limits.max_header_bytes + READ_BUFFER;
    assert_eq!(bound, 73_728);
    eprintln!("held {held} bytes of the bound of {bound}");
    assert!(bound * 56 <= body.len());
    assert!(
        held <= bound,
        "crate documentation of surf-fetch, \"Memory\": a Post of {} bytes sent twice through a \
         308 held {held} bytes over the whole fetch, {} bytes over the header limit plus one \
         read buffer ({bound})",
        body.len(),
        held.saturating_sub(bound)
    );
}
