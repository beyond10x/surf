//! Fetch one `http:` or `https:` address over HTTP/1.1, with the Rust standard library and, for
//! `https:`, the TLS 1.3 client of `surf-tls`.
//!
//! [`fetch`] takes an address as text and a set of [`Limits`], sends a `GET` request with
//! `Host` and `Connection: close`, follows redirects with one more such request each, and
//! returns a [`Response`] (status code, media type, body bytes, the address it came from) or a
//! [`Failure`] with one of ten reasons: [`Failure::Unreachable`],
//! [`Failure::UnsupportedScheme`], [`Failure::MalformedResponse`],
//! [`Failure::TooManyRedirects`], [`Failure::ForbiddenDestination`], and the five of TLS
//! (`docs/design/tls.md`, item 7): [`Failure::CertificateNotTrusted`],
//! [`Failure::CertificateExpired`], [`Failure::CertificateNameMismatch`],
//! [`Failure::TlsVersionUnsupported`] and [`Failure::TlsHandshakeFailed`].
//!
//! An `https:` request connects as an `http:` one does, through the [`Network`] and under the
//! rule of "Destinations", and then runs the TLS 1.3 handshake of
//! `surf_tls::connection::Connection` on that connection, for the host of the address, with the
//! server's certificates checked by `surf_tls::path::PathVerifier` over the trust anchors of the
//! [`Request`] ([`Request::anchors`]: the bundled list of `surf-tls`, with the anchors of an
//! extra file when the caller added them; [`fetch`] uses the bundled list) at the system clock.
//! It offers ALPN `http/1.1`, and sends `server_name` for a host that is a name. The HTTP of the
//! request and the response is then the same as over `http:`, written to and read from the TLS
//! connection. A redirect from `http:` to `https:`, and from `https:` to `http:`, is followed
//! like any other. A failure of the handshake is one of the five reasons of TLS, whose detail is
//! the text of `surf_tls::error::Error` (a value the server chose only as
//! `surf_tls::error::excerpt` shows it); a server that never finishes the handshake, a
//! connection that ends during it, and one that ends without `close_notify` before the response
//! is complete are [`Failure::Unreachable`]. An empty set of anchors is
//! [`Failure::CertificateNotTrusted`] before any connection.
//!
//! [`send_through`] is [`send`] over a [`Network`] of the caller's: where names are resolved and
//! connections made. [`send`] uses [`OsNetwork`], the operating system's resolver and
//! [`TcpStream::connect_timeout`]; a test passes a resolver and a transport of its own.
//! [`send_from`] is [`send_through`] for a request whose [`Origin`] is a page at a public
//! address ("Destinations").
//!
//! [`send`] does the same for a [`Request`]: an address, a [`Method`] (`GET` or `POST`), and for
//! a `POST` an optional body and its media type. [`fetch`] is [`send`] of a `GET` without a body.
//! A `POST` sends `Host`, `Connection: close`, `Content-Type: <media type>` when the request has
//! one, and `Content-Length` always (`0` without a body, as RFC 9110 section 8.6 says a user agent
//! normally does), then the body bytes unchanged. A `GET` sends neither field and no body.
//!
//! A redirect changes the method as the Fetch Standard's "HTTP-redirect fetch" does: after a 301,
//! 302 or 303 to a `POST`, the next request is a `GET` without a body, `Content-Type` or
//! `Content-Length`, and stays one for the rest of the chain; a 307 or 308 repeats the method, the
//! body and the media type. A 303 after a `GET` stays a `GET`.
//!
//! # Cookies
//!
//! A [`Request`] with a jar ([`Request::jar`], a `surf_cookie::Jar`) sends, on every hop, the
//! `Cookie` field the jar gives for the hop's address, its method and whether it is same-site
//! with the request's initiator ([`Request::initiator`], kept by every hop of a redirect), and
//! cross-site on every hop after the chain has been on another site (`surf_cookie::Chain`); one
//! field after `Host` and `Connection`, none when no cookie matches.
//! Every `Set-Cookie` field of each response, redirects included, goes into the jar for that
//! response's address before the next hop is sent; one that is not UTF-8 is dropped. A
//! `Set-Cookie` field longer than the header limit ends the fetch as every such field does
//! ([`Failure::MalformedResponse`]), and nothing of that response goes into the jar. With a jar
//! the address is parsed once, by `surf_url::Url::parse`: the connection, the `Host` field and
//! the jar's choice of cookies all come from that parse's serialization, on the first request as
//! on every redirect, so a cookie goes only to the host it was chosen for (for
//! `http://a.test\b.test/` that is `a.test`, as a browser reads it). An address
//! `surf_url::Url::parse` does not parse is sent as above, with no cookie, and keeps none. The
//! rules of the jar (draft-ietf-httpbis-rfc6265bis-22) are those of the crate `surf-cookie`. A
//! request without a jar sends no `Cookie` field and keeps no cookie.
//!
//! The wire format follows HTTP/1.1 (RFC 9112, message syntax; RFC 9110, semantics) for what
//! this crate does. No section of either is claimed complete. Where a rule is followed, the
//! comment beside the code names the section it was checked against.
//!
//! # Limits and how each is enforced
//!
//! The bytes come from an untrusted server. Every limit below ends the fetch with a [`Failure`];
//! none of them is a hang or an unbounded allocation, and no input makes this crate panic.
//!
//! - **Connect timeout** ([`Limits::connect_timeout`], default 10 s): bounds name resolution
//!   and connecting together, over every address the host resolves to and summed over every
//!   request of the fetch, redirects included. Each
//!   [`std::net::TcpStream::connect_timeout`] gets the time that is left. A host that is not an
//!   IP address is resolved by the operating system's resolver through
//!   [`std::net::ToSocketAddrs`] on a thread of its own, waited for through a channel with
//!   [`std::sync::mpsc::Receiver::recv_timeout`]. When the time runs out first, that thread is
//!   abandoned, not stopped: it ends when the resolver returns, and its answer is dropped.
//!   Running out is [`Failure::Unreachable`]. This limit has no test: no address and no name on
//!   the local machine stalls a connect or a resolution reliably, and a test may not reach
//!   outside the machine.
//! - **Read timeout** ([`Limits::read_timeout`], default 30 s): bounds the whole exchange after
//!   the connection is made, from sending the request to the last byte of the response, summed
//!   over every request of the fetch, redirects included. For `https:` the TLS handshake is
//!   inside it: it starts when the connection is made, and the handshake's reads and writes
//!   share the deadline of the exchange that follows, so one fetch over any mix of `http:` and
//!   `https:` requests still lasts at most the two timeouts. Each
//!   write and each read on the socket is given the time that is left, through
//!   [`std::net::TcpStream::set_write_timeout`] and [`std::net::TcpStream::set_read_timeout`].
//!   Running out is [`Failure::Unreachable`], the same failure a server that sends nothing
//!   gives. Sending a request body, again after a 307 or 308 that repeats it, counts against it
//!   as every write does. For a redirect, the time from sending its request to the end of resolving its
//!   `Location` (the parse of the `Location` and of the address it resolves to) counts against
//!   it, so the next request gets only what is left.
//! - **Header bytes** ([`Limits::max_header_bytes`], default 64 KiB): every byte of the status
//!   line and the header field lines, line terminators and interim (1xx) responses included, is
//!   counted against it, and a line is never read past what is left of it. Each response of a
//!   fetch, every redirect included, has a budget of its own. Exceeding it is
//!   [`Failure::MalformedResponse`].
//! - **Body bytes** ([`Limits::max_body_bytes`], default 16 MiB): a `Content-Length` larger than
//!   the limit fails before any body byte is read; a chunk whose size would take the body past
//!   the limit fails before the chunk is read; a body read until the connection closes fails as
//!   soon as it holds one byte more than the limit. Each is [`Failure::MalformedResponse`].
//! - **Request body bytes** ([`Limits::max_request_body_bytes`], default 16 MiB): a request body
//!   longer than the limit is [`Failure::Unreachable`] before the host is resolved, with no
//!   connection.
//! - **Request checks**: after the address checks and before the host is resolved, each with no
//!   connection and [`Failure::Unreachable`] whose detail names the check (as for an address
//!   this crate cannot use: the specification has no reason for a request that cannot be sent):
//!   a `GET` with a body or a media type; a media type that is not `type "/" subtype`, each a
//!   token (RFC 9110 section 5.6.2) of at most 127 bytes (RFC 6838 section 4.2), which refuses
//!   CR, LF, `;` and so any parameter and any header injection; and the request body limit above.
//! - **Chunk-size line**: a chunk-size line, extensions included, longer than 1024 bytes is
//!   [`Failure::MalformedResponse`]. Chunk sizes are parsed with checked arithmetic.
//! - **Redirects**: a response with a redirect status (301, 302, 303, 307 or 308) and a
//!   `Location` field is followed, at most 20 times in one fetch; a redirect after the twentieth
//!   (the 21st response of a chain of 21, or of a loop) is [`Failure::TooManyRedirects`]. The
//!   body of a followed redirect is not read: its connection is closed after its header
//!   section. `Location` is read as UTF-8 and resolved with `surf_url::Url::parse` against the
//!   address of the response that carried it (the Fetch Standard, "location URL"); a resolved
//!   URL without a fragment takes the fragment of that address, and the fragment is never sent.
//!   A `Location` that is not UTF-8, does not parse (a host outside ASCII included: no IDNA), or
//!   comes in more than one field, and a resolved address longer than [`MAX_ADDRESS_BYTES`]
//!   (64 KiB), are [`Failure::MalformedResponse`]. The resolved address then goes through the
//!   same checks as the address passed to [`fetch`] or [`send`] (see "Addresses"). Each of
//!   these failures ends the fetch without a further connection, and so does a redirect after
//!   the twentieth.
//!   The order of the checks: the `Location` and the address bound, then the address, then the
//!   number of redirects, as the Fetch Standard's "HTTP-redirect fetch" checks the location URL,
//!   then its scheme, then the redirect count.
//!
//! - **Destinations**: see "Destinations" below. A refused destination ends the fetch with
//!   [`Failure::ForbiddenDestination`], before any connection to it.
//! - **TLS** (`docs/design/tls.md`, item 9; `surf-tls` holds each): a handshake message of at
//!   most 65,535 bytes; at most 64 certificates in the server's Certificate message; at most 6
//!   intermediates on a path; per verification at most 100 signature checks, 25,000 candidates
//!   considered and 250,000 name-constraint comparisons; one HelloRetryRequest; at most
//!   23,726,566 records under one AES-GCM write key; at most 32 records in a row that carry no
//!   application data. Each refusal is one of the five reasons of TLS.
//!
//! So one fetch lasts at most the connect timeout plus the read timeout, whatever the number of
//! redirects: 40 s at the defaults.
//!
//! # Destinations
//!
//! The host of every request of a fetch is resolved once (an IP address is its own), and the
//! addresses resolved are the ones checked and then dialed, in their order: no second
//! resolution comes between the check and the connection. An address is *local* when it is
//! loopback (127.0.0.0/8, `::1`), private (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16),
//! link-local (169.254.0.0/16, `fe80::/10`), unique-local (`fc00::/7`), shared (100.64.0.0/10)
//! or unspecified (0.0.0.0, `::`), or an IPv6 address that embeds an IPv4 address of one of
//! these: IPv4-mapped (`::ffff:a.b.c.d`), IPv4-compatible (`::a.b.c.d`), NAT64 (`64:ff9b::/96`)
//! or 6to4 (`2002::/16`); every other address is *public*.
//!
//! The first request of a fetch may go to any address: going to a local address on purpose is a
//! use of surf. When the first request connected to a public address, a request that follows a
//! redirect is refused when its host resolves to a local address, any one of those resolved,
//! also when a public one comes first: [`Failure::ForbiddenDestination`], with no connection to
//! any of them. So a page on the public web cannot send a request into the loopback or private
//! network of the machine, by an address literal or by a name that resolves to a public address
//! first and a local one later. When the first request connected to a local address, the chain
//! follows its redirects as before, to local and to public addresses. The port is not looked at.
//!
//! A request whose [`Origin`] is [`Origin::PublicPage`], a link or a form of a page at a public
//! address (`story:agent-press-submit`, "As decided after the adversary's pass", decision 1), is
//! held to that rule from its first request on: a host that resolves to a local address, any one
//! of those resolved, is refused with [`Failure::ForbiddenDestination`] before any connection and
//! before a byte is sent, and so is a local IP address. An address given by the one who fetches
//! ([`Origin::Given`], what [`send`] and [`send_through`] use) keeps the rule above.
//!
//! A zero [`Duration`] for either timeout leaves no time and fails every fetch with
//! [`Failure::Unreachable`]. A duration too large to add to the current [`Instant`] sets no
//! deadline, and each read, write and connect waits that duration as the standard library
//! allows.
//!
//! # Memory
//!
//! What a fetch holds at once depends on the moment; a body is held only in the first.
//!
//! A request body is the caller's: [`send`] borrows it and writes it from the caller's slice,
//! after the head, and allocates no copy of it, also when a 307 or 308 repeats it. None of the
//! sums below counts it. So while it is written the fetch holds what it holds without it: a
//! `POST` of 4 MiB sent twice through a 308, with short addresses and a response of 2 bytes,
//! holds at most the header limit and one read buffer over the whole fetch, 73,728 bytes at the
//! defaults (8,761 measured).
//!
//! - **While the body of the last response is read**, at most the sum of these, 16,917,505
//!   bytes at the defaults:
//!   - the header limit, 64 KiB: of the head only the media type is kept, at most the header
//!     limit; the head itself is dropped before the first body byte is read;
//!   - the body limit and one byte, 16 MiB + 1: the body buffer grows by doubling, as a
//!     [`Vec`] does, but never past the declared length or the body limit, and a body read
//!     until the connection closes takes one byte more to tell it is too long;
//!   - one read buffer, 8 KiB: the bytes read and not yet consumed, a buffer cut back to one
//!     read once the head is read;
//!   - one chunk-size line, 1 KiB;
//!   - after a redirect, the address of the response, at most [`MAX_ADDRESS_BYTES`], 64 KiB.
//!     Without a redirect the response's address is a copy of the address passed to [`fetch`]
//!     or [`send`], made once the body is complete. The address parsed with `surf-url`, its
//!     fragment, the request target and the request head are dropped before the body is read.
//! - **While a `Location` is resolved**, no body is held: the head of the redirect, the URL it
//!   is resolved against parsed with `surf-url` (its path takes 24 bytes for each segment
//!   besides its bytes), and what parsing the `Location` takes (4 bytes for each code point of
//!   it, 24 bytes for each path segment). Measured by counting allocations, for a `Location` at
//!   the default header limit of the costliest shape found, a path of empty segments
//!   (`/a////…`): at most 30 times the header limit, 1,966,080 bytes, when it is resolved
//!   against a short address passed to [`fetch`] (29.003 times measured); at most 54 times,
//!   3,538,944 bytes, when it is resolved against a URL of 64 KiB of that shape that the
//!   redirect before led to (53.003 times measured).
//! - **While a head is read**, no body is held: the bytes read and not yet consumed, at most
//!   the larger of the header limit and one read buffer, plus one read; the fields parsed so
//!   far, each taking 48 bytes besides its name and value; and after a redirect the address of
//!   the response, the request target and the URL parsed as above. No bound is stated for
//!   this moment: a header section of many short fields holds many times the header limit.
//!
//! An `https:` request holds, beside the above: the verifier's copy of the trust anchors, made
//! before its first connection; the TLS connection, whose record layer holds at most one record
//! and one handshake message (`surf_tls::record`: about 2^14 + 256 and 65,535 + 2^14 bytes);
//! and, during the handshake, the server's certificates while they are verified. Measured by
//! counting allocations on the fetch's thread (the module `https` of the tests): a `GET` of a
//! 64 KiB body from the TLS test server (a chain of two certificates, the bundled anchors and
//! one more) held 173,747 bytes at its peak, the same `GET` over `http:` 73,738, so the TLS
//! connection added 100,009 bytes; that test holds it to at most 131,072. No bound is stated
//! for a server's chain at the bounds of item 9 of `docs/design/tls.md` (a Certificate message
//! of 65,535 bytes, up to 64 certificates): what parsing and verifying it takes was not
//! measured.
//!
//! A request with a jar holds, beside the above, while the head of a hop is written and read,
//! the address of the hop parsed with `surf-url` and the `Cookie` field, at most 8,192 bytes;
//! both are dropped before a body is read. The jar is the caller's, and `surf-cookie` states
//! its bound.
//!
//! # What it does not do
//!
//! It is not a complete HTTP client. It does not do any scheme other than `http:` and `https:`
//! (a redirect to one is [`Failure::UnsupportedScheme`]), TLS 1.2 or below (a server limited to
//! it fails with [`Failure::TlsVersionUnsupported`] or [`Failure::TlsHandshakeFailed`]), client
//! certificates, TLS session resumption (a ticket the server sends is read and dropped), HTTP/2,
//! methods other than `GET` and `POST`,
//! `Expect: 100-continue` (a `POST` sends its body at once, after the head), chunked request
//! bodies, a request media type with parameters, request header fields other than `Host`,
//! `Connection`, `Cookie`, `Content-Type` and `Content-Length` (no `Origin`, no `Referer`),
//! persistent connections (every fetch opens and closes one connection), content codings (the
//! body is returned as sent; no `Accept-Encoding` is sent, and no `Content-Encoding` is decoded),
//! transfer codings other than `chunked`, trailers (the chunked body ends at its last chunk; the
//! trailer section is not read), proxies, caching, `Refresh`, or URL parsing beyond
//! splitting an address into host, port and request target and resolving a `Location`.
//!
//! It does not check revocation: a revoked certificate that has not expired loads.
//!
//! A server that answers and closes before it read a request body ends the fetch in one of two
//! ways, and a caller may rely on neither. What decides is whether the write of the request met
//! the closed connection. When it did not (as a rule, a body small enough for the socket's
//! buffers: it is written without an error), the answer is read and returned as a [`Response`].
//! When it did (as a rule, a body larger than those buffers), the write fails: that is
//! [`Failure::Unreachable`], as every failure to send the request is, and the answer is not read.
//!
//! # Addresses
//!
//! An address is `http://host[:port][/path][?query][#fragment]` or the same with `https://`,
//! the scheme in any case. The port defaults to 80 for `http:` and 443 for `https:`, the
//! fragment is not sent, and an empty path is sent as `/`. An address that starts with neither
//! `http:` nor `https:` is [`Failure::UnsupportedScheme`] and no connection is attempted. An
//! address of either scheme this crate cannot use (no `//`, an empty host, a host in
//! brackets that is not an IPv6 address, a bad port, user information, or a byte that is not
//! printable ASCII) is [`Failure::Unreachable`], also without a connection: the specification
//! has no reason for an address that is not one. When a redirect arrives, the address passed to
//! [`fetch`] or [`send`] is parsed with `surf_url::Url::parse` to resolve the `Location` against; one that
//! does not parse is [`Failure::Unreachable`] for the same reason. A request with a jar
//! ("Cookies") is checked as above and then sent to its address as `surf_url::Url::parse` reads
//! it.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use surf_cookie::Jar;
use surf_tls::connection::Connection;
use surf_tls::der::Time;
use surf_tls::error::{AlertDescription, Error as TlsError};
use surf_tls::path::PathVerifier;
use surf_tls::verify::CertificateRejection;

pub use surf_tls::anchors;
pub use surf_tls::anchors::{AnchorError, Anchors};

/// The media type of a response without a usable `Content-Type`.
const DEFAULT_MEDIA_TYPE: &str = "application/octet-stream";

/// The longest chunk-size line, extensions and line terminator included.
const MAX_CHUNK_LINE_BYTES: usize = 1024;

/// The most bytes one read from the connection asks for.
const READ_CHUNK_BYTES: usize = 8 * 1024;

/// The bounds a fetch stays within. Every field is public so a caller, or a test, can shorten
/// one and keep the [`Default`] of the others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// How long resolving the host and connecting to one of its addresses may take together.
    /// Default 10 s.
    pub connect_timeout: Duration,
    /// How long sending the request and receiving the whole response may take together, once
    /// the connection is made. Default 30 s.
    pub read_timeout: Duration,
    /// The most bytes the status line and header section may take, interim (1xx) responses
    /// included. Default 64 KiB.
    pub max_header_bytes: usize,
    /// The most bytes the body may take after the chunked coding is removed. Default 16 MiB.
    pub max_body_bytes: usize,
    /// The most bytes the body of a request may take. A longer one is not sent. Default 16 MiB.
    pub max_request_body_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(30),
            max_header_bytes: 64 * 1024,
            max_body_bytes: 16 * 1024 * 1024,
            max_request_body_bytes: 16 * 1024 * 1024,
        }
    }
}

/// The method of a [`Request`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// `GET`, without a body.
    Get,
    /// `POST`, with an optional body and its media type.
    Post,
}

impl Method {
    fn name(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
        }
    }
}

/// A request to send, borrowed from the caller: nothing of it is copied, the body least of all.
///
/// Its [`Debug`](fmt::Debug) text shows the address, the method, the media type and the
/// initiator, the body by its length only, never its bytes, since a form's body can carry a
/// password, the anchors by their count, and the jar by the count of its cookies, never a
/// cookie: `Request { address: "http://example.org/login", method: Post, body: Some(16 bytes),
/// media_type: Some("application/x-www-form-urlencoded"), anchors: None, jar: Some(Jar {
/// cookies: 1 }), initiator: Some("http://example.org") }`.
#[derive(Clone, Copy)]
pub struct Request<'a> {
    /// The address, as for [`fetch`].
    pub address: &'a str,
    /// The method.
    pub method: Method,
    /// The body, sent as it is. Only a [`Method::Post`] may have one.
    pub body: Option<&'a [u8]>,
    /// The media type of the body, `type/subtype` without parameters, sent as `Content-Type`.
    /// Only a [`Method::Post`] may have one.
    pub media_type: Option<&'a str>,
    /// The trust anchors the server of an `https:` request is authenticated against: the
    /// bundled list with the anchors of an extra file ([`Anchors::with_extra_pem`]). `None` is
    /// the bundled list alone ([`Anchors::bundled`]), read at the first `https:` request of the
    /// fetch.
    pub anchors: Option<&'a Anchors>,
    /// The cookie jar of the session the request belongs to: each hop sends the `Cookie` field
    /// the jar gives for its address, and every `Set-Cookie` field of each response goes into
    /// it (the crate documentation, "Cookies"). `None` sends no cookie and keeps none.
    pub jar: Option<&'a Jar>,
    /// The serialized origin of the document that started the navigation, such as
    /// `http://127.0.0.1:8080`, which decides whether a hop is same-site for the jar
    /// ([`surf_cookie::site`]). `None` for a navigation nobody's document started.
    pub initiator: Option<&'a str>,
}

impl fmt::Debug for Request<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("address", &self.address)
            .field("method", &self.method)
            .field("body", &self.body.map(|body| BodyLength(body.len())))
            .field("media_type", &self.media_type)
            .field(
                "anchors",
                &self.anchors.map(|anchors| AnchorCount(anchors.count())),
            )
            .field("jar", &self.jar)
            .field("initiator", &self.initiator)
            .finish()
    }
}

/// The number of trust anchors, which is all the [`Debug`](fmt::Debug) text of a [`Request`]
/// shows of them: `<n> anchors`.
struct AnchorCount(usize);

impl fmt::Debug for AnchorCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} anchors", self.0)
    }
}

/// The length of a request body, which is all the [`Debug`](fmt::Debug) text of a [`Request`]
/// shows of it: `<n> bytes`.
struct BodyLength(usize);

impl fmt::Debug for BodyLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} bytes", self.0)
    }
}

/// A complete response, whatever its status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// The status code, from 100 to 599. Never a 1xx: interim responses are skipped.
    pub status: u16,
    /// The type and subtype of `Content-Type` in lower case without parameters, such as
    /// `text/html`; `application/octet-stream` when the field is absent or not a media type.
    pub media_type: String,
    /// The body bytes, with the chunked coding removed and nothing else decoded.
    pub body: Vec<u8>,
    /// The address the response came from: the address passed to [`fetch`] or [`send`], byte
    /// for byte, when nothing redirected, else the serialization of the URL the last redirect
    /// led to. At most [`MAX_ADDRESS_BYTES`] bytes after a redirect.
    pub url: String,
    /// The method of the request this response answers: the method of the [`Request`], or
    /// [`Method::Get`] after a redirect that changed it (the crate documentation, "Redirects").
    pub method: Method,
}

/// Why a fetch ended without a response. Each variant carries a human-readable detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// No connection could be made, or it ended before the response was complete: the address
    /// is unusable, the host does not resolve, the connection is refused or times out, the
    /// server sends nothing for longer than the read timeout, or the connection closes early.
    Unreachable(String),
    /// The address, or the address a redirect leads to, does not start with the `http:`
    /// scheme. No connection to it was attempted.
    UnsupportedScheme(String),
    /// Bytes arrived that are not an HTTP/1.1 response, or the header or body limit was
    /// exceeded, or a redirect's `Location` cannot be followed: it is not UTF-8, does not
    /// parse, comes twice, or resolves to more than [`MAX_ADDRESS_BYTES`] bytes.
    MalformedResponse(String),
    /// A redirect arrived after 20 redirects of the same fetch were followed.
    TooManyRedirects(String),
    /// A redirect led from a public address to a local one (the crate documentation,
    /// "Destinations"). No connection to it was attempted.
    ForbiddenDestination(String),
    /// The certificates of an `https:` server lead to no trust anchor: the detail names the
    /// check that failed (`docs/design/tls.md`, item 7). Also an empty set of anchors, refused
    /// before any connection.
    CertificateNotTrusted(String),
    /// A certificate on the path of an `https:` server is outside its validity at the time of
    /// the fetch.
    CertificateExpired(String),
    /// No subjectAltName entry of the `https:` server's certificate matches the host, or the
    /// host is one no certificate can match (refused before a byte is sent).
    CertificateNameMismatch(String),
    /// The `https:` server does not speak TLS 1.3: it sent `protocol_version` before its
    /// ServerHello, or its ServerHello selected TLS 1.2 or below.
    TlsVersionUnsupported(String),
    /// The TLS handshake, or the TLS connection after it, failed otherwise: an alert the server
    /// sent, or one surf sent for a violation of the protocol.
    TlsHandshakeFailed(String),
}

impl Failure {
    /// The human-readable detail the failure carries.
    pub fn detail(&self) -> &str {
        match self {
            Failure::Unreachable(detail)
            | Failure::UnsupportedScheme(detail)
            | Failure::MalformedResponse(detail)
            | Failure::TooManyRedirects(detail)
            | Failure::ForbiddenDestination(detail)
            | Failure::CertificateNotTrusted(detail)
            | Failure::CertificateExpired(detail)
            | Failure::CertificateNameMismatch(detail)
            | Failure::TlsVersionUnsupported(detail)
            | Failure::TlsHandshakeFailed(detail) => detail,
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Failure::Unreachable(_) => "unreachable",
            Failure::UnsupportedScheme(_) => "unsupported scheme",
            Failure::MalformedResponse(_) => "malformed response",
            Failure::TooManyRedirects(_) => "too many redirects",
            Failure::ForbiddenDestination(_) => "forbidden destination",
            Failure::CertificateNotTrusted(_) => "certificate not trusted",
            Failure::CertificateExpired(_) => "certificate expired",
            Failure::CertificateNameMismatch(_) => "certificate name mismatch",
            Failure::TlsVersionUnsupported(_) => "TLS version unsupported",
            Failure::TlsHandshakeFailed(_) => "TLS handshake failed",
        };
        write!(f, "{reason}: {}", self.detail())
    }
}

impl std::error::Error for Failure {}

/// Fetch `address` with `GET` requests, following redirects, and return the response, or why
/// there is none: [`send`] of a [`Method::Get`] without a body.
///
/// Blocks the calling thread for as long as the [`Limits`] allow; see the crate documentation
/// for how each is enforced.
pub fn fetch(address: &str, limits: &Limits) -> Result<Response, Failure> {
    send(
        &Request {
            address,
            method: Method::Get,
            body: None,
            media_type: None,
            anchors: None,
            jar: None,
            initiator: None,
        },
        limits,
    )
}

/// Send `request`, following redirects, and return the response, or why there is none.
///
/// The checks of the request come after those of the address and before the host is resolved;
/// a redirect changes the method as the crate documentation says. Blocks the calling thread for
/// as long as the [`Limits`] allow.
pub fn send(request: &Request<'_>, limits: &Limits) -> Result<Response, Failure> {
    send_through(request, limits, &OsNetwork)
}

/// Where a fetch resolves names and makes connections. [`OsNetwork`] is the operating system's;
/// a test supplies its own, to give a name the addresses it chooses and to see every address
/// dialed. The rule of the crate documentation, "Destinations", is applied between the two
/// methods, by the fetch, whatever the network.
pub trait Network {
    /// The addresses `host` (a name, never an IP address) resolves to, with `port`, within
    /// `timeout`. A failure is [`Failure::Unreachable`] as a rule.
    fn resolve(&self, host: &str, port: u16, timeout: Duration)
        -> Result<Vec<SocketAddr>, Failure>;

    /// A connection to `address`, one the fetch has checked, within `timeout`.
    fn connect(&self, address: &SocketAddr, timeout: Duration) -> std::io::Result<TcpStream>;
}

/// The operating system's network: [`std::net::ToSocketAddrs`] on a thread of its own, waited
/// for at most the timeout, and [`TcpStream::connect_timeout`].
#[derive(Debug, Clone, Copy, Default)]
pub struct OsNetwork;

impl Network for OsNetwork {
    fn resolve(
        &self,
        host: &str,
        port: u16,
        timeout: Duration,
    ) -> Result<Vec<SocketAddr>, Failure> {
        resolve_name(host, port, timeout)
    }

    fn connect(&self, address: &SocketAddr, timeout: Duration) -> std::io::Result<TcpStream> {
        TcpStream::connect_timeout(address, timeout)
    }
}

/// [`send`] over `network`: names are resolved and connections made by it.
pub fn send_through(
    request: &Request<'_>,
    limits: &Limits,
    network: &dyn Network,
) -> Result<Response, Failure> {
    send_from(request, limits, network, Origin::Given)
}

/// Where the first request of a fetch comes from, for the rule of the crate documentation,
/// "Destinations".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Origin {
    /// An address the one who fetches gave, such as one an agent typed: the first request may go
    /// to any address.
    #[default]
    Given,
    /// A link or a form of a page at a public address: the first request, and every one after
    /// it, goes to public addresses only.
    PublicPage,
}

/// [`send_through`] for a request that comes from `origin`: with [`Origin::PublicPage`] a host
/// of the first request that resolves to a local address is refused before any connection.
pub fn send_from(
    request: &Request<'_>,
    limits: &Limits,
    network: &dyn Network,
    origin: Origin,
) -> Result<Response, Failure> {
    let address = request.address;
    let mut target = Target::parse(address)?;
    check(request, limits)?;
    // What the next request sends; a redirect may turn it into a GET without a body.
    let mut outgoing = Outgoing {
        method: request.method,
        body: request.body,
        media_type: request.media_type,
    };
    // The response URL once a redirect is followed; until then the asked address byte for byte,
    // copied only into a response.
    let mut url: Option<String> = None;
    // The current URL parsed with surf-url, and its fragment: from the first request on for a
    // fetch with a jar, otherwise once a redirect needs them.
    let mut current: Option<(surf_url::Url, Option<String>)> = None;
    // The crate documentation, "Cookies": with a jar, the address is parsed once, and the
    // connection, the `Host` field and the jar's lookup all come from that parse.
    if request.jar.is_some() {
        if let Ok(parsed) = surf_url::Url::parse(address, None) {
            target = Target::parse(&parsed.href())?;
            let fragment = parsed.fragment().map(str::to_owned);
            current = Some((parsed, fragment));
        }
    }
    let mut redirects = 0;
    // The addresses of the chain so far, for whether a hop is same-site for cookies.
    let mut chain = surf_cookie::Chain::new();
    let mut connect_left = limits.connect_timeout;
    let mut read_left = limits.read_timeout;
    // Whether the first request connected to a local address; none before it connected. A
    // request from a public page counts as one whose first request went to a public address.
    let mut started_local: Option<bool> = match origin {
        Origin::Given => None,
        Origin::PublicPage => Some(false),
    };
    // The verifier of the fetch's `https:` requests, made before the first one connects.
    let mut verifier: Option<PathVerifier> = None;
    loop {
        if target.scheme == Scheme::Https && verifier.is_none() {
            verifier = Some(path_verifier(request.anchors)?);
        }
        let connecting = Instant::now();
        // The crate documentation, "Destinations": after a first request to a public address,
        // and from a public page, every request must resolve to public addresses only.
        let public_only = match (started_local, redirects) {
            (Some(false), 0) => Some(Refusing::FromPublicPage),
            (Some(false), _) => Some(Refusing::Redirect),
            _ => None,
        };
        let (stream, dialed) = connect(&target, connect_left, network, public_only)?;
        if started_local.is_none() {
            started_local = Some(is_local(dialed.ip()));
        }
        connect_left = connect_left.saturating_sub(connecting.elapsed());
        // From here the read timeout runs: the TLS handshake and the exchange share it.
        let exchanging = Instant::now();
        let stream = match target.scheme {
            Scheme::Http => Stream::Plain(Timed::new(stream, read_left)),
            Scheme::Https => {
                let Some(verifier) = verifier.as_ref() else {
                    return Err(unreachable("no verifier was made for the https: request"));
                };
                // Each flight of the handshake is one write; without the delay of Nagle's
                // algorithm the request that follows the client's Finished goes at once.
                let _ = stream.set_nodelay(true);
                secure(Timed::new(stream, read_left), &target.host, verifier)?
            }
        };
        // The crate documentation, "Cookies": the jar sees the parse this hop connected by; a
        // fetch without a jar parses nothing for it.
        let hop_url = request.jar.and(current.as_ref().map(|(url, _)| url));
        let cookie = match (request.jar, hop_url) {
            (Some(jar), Some(url)) => jar.cookie_field(
                url,
                chain.site(request.initiator, url),
                match outgoing.method {
                    Method::Get => surf_cookie::Method::Get,
                    Method::Post => surf_cookie::Method::Post,
                },
                SystemTime::now(),
            ),
            _ => None,
        };
        let (head, wire) = exchange(&target, &outgoing, cookie.as_deref(), stream, limits)?;
        drop(cookie);
        if let (Some(jar), Some(url)) = (request.jar, hop_url) {
            // A field that is not UTF-8 is no cookie the jar can read: it is dropped.
            for set_cookie in head.values(b"set-cookie") {
                if let Ok(set_cookie) = std::str::from_utf8(set_cookie) {
                    jar.receive(url, set_cookie, SystemTime::now());
                }
            }
        }
        let status = head.status;
        // The request is sent: of the address, only what a response needs is kept from here
        // (the crate documentation, "Memory").
        drop(target);
        let Some(location) = redirect_location(&head)? else {
            drop(current);
            let (status, media_type, body) = read_rest(wire, head, limits.max_body_bytes)?;
            return Ok(Response {
                status,
                media_type,
                body,
                url: url.unwrap_or_else(|| address.to_owned()),
                method: outgoing.method,
            });
        };
        // The body of a redirect is not read: the connection closes here.
        drop(wire);
        // The address of the redirect is no response's: it is needed only parsed, as `current`
        // or, at the first redirect, the address passed.
        drop(url.take());
        let (base, fragment) = match current.take() {
            Some(current) => current,
            None => {
                let base = surf_url::Url::parse(address, None).map_err(|e| {
                    unreachable(format!(
                        "a redirect cannot be resolved against the address: {e}"
                    ))
                })?;
                let fragment = base.fragment().map(str::to_owned);
                (base, fragment)
            }
        };
        let (next, next_url, next_fragment) = resolve_location(location, &base, fragment)?;
        target = Target::parse(&next_url)?;
        // The exchange is charged up to here, so resolving the Location counts against the
        // read timeout too.
        read_left = read_left.saturating_sub(exchanging.elapsed());
        // A redirect after the twentieth: the Fetch Standard, "HTTP-redirect fetch", step 8.
        if redirects == MAX_REDIRECTS {
            return Err(Failure::TooManyRedirects(format!(
                "a redirect arrived after {MAX_REDIRECTS} redirects were followed"
            )));
        }
        redirects += 1;
        // The Fetch Standard, "HTTP-redirect fetch", step 12: a 301 or 302 after a POST, and a
        // 303 after any method but GET (or HEAD), continue as GET with a null body and without
        // the request-body-header names, Content-Type among them. With GET and POST only, that
        // is every 301, 302 and 303 after a POST.
        if outgoing.method == Method::Post && matches!(status, 301..=303) {
            outgoing = Outgoing {
                method: Method::Get,
                body: None,
                media_type: None,
            };
        }
        url = Some(next_url);
        current = Some((next, next_fragment));
    }
}

/// What one request of a fetch sends besides its target: the method, the body, its media type.
struct Outgoing<'a> {
    method: Method,
    body: Option<&'a [u8]>,
    media_type: Option<&'a str>,
}

/// The longest type or subtype of a request's media type: RFC 6838 section 4.2.
const MAX_MEDIA_TYPE_NAME_BYTES: usize = 127;

/// The checks of `request` before its host is resolved; each failure names its check.
fn check(request: &Request<'_>, limits: &Limits) -> Result<(), Failure> {
    if request.method == Method::Get && (request.body.is_some() || request.media_type.is_some()) {
        return Err(unreachable(
            "the request is a GET with a body or a media type, which is not sent",
        ));
    }
    if let Some(media_type) = request.media_type {
        let parts = media_type.split_once('/');
        let usable = parts.is_some_and(|(kind, subtype)| {
            [kind, subtype]
                .iter()
                .all(|name| is_token(name.as_bytes()) && name.len() <= MAX_MEDIA_TYPE_NAME_BYTES)
        });
        if !usable {
            return Err(unreachable(format!(
                "the media type of the request is not type/subtype, each a token of at most \
                 {MAX_MEDIA_TYPE_NAME_BYTES} bytes"
            )));
        }
    }
    if let Some(body) = request.body {
        if body.len() > limits.max_request_body_bytes {
            return Err(unreachable(format!(
                "the request body of {} bytes is longer than the limit of {} bytes",
                body.len(),
                limits.max_request_body_bytes
            )));
        }
    }
    Ok(())
}

/// The most redirects one fetch follows: the Fetch Standard, "HTTP-redirect fetch".
const MAX_REDIRECTS: usize = 20;

/// The longest address a redirect may lead to, in bytes: 64 KiB. A `Location` that resolves to
/// a longer one ends the fetch with [`Failure::MalformedResponse`]. The address passed to
/// [`fetch`] is not checked against it.
pub const MAX_ADDRESS_BYTES: usize = 65_536;

/// Send the request for `target` on `stream`, with the `Cookie` field `cookie` when there is
/// one, and read the head of the final (not interim) response, within the deadline of `stream`.
/// Returns the head and the connection with what is left of it.
fn exchange(
    target: &Target,
    outgoing: &Outgoing<'_>,
    cookie: Option<&str>,
    mut stream: Stream,
    limits: &Limits,
) -> Result<(Head, Wire<Stream>), Failure> {
    // Request target in origin-form and Host as the address's authority: RFC 9112 section
    // 3.2.1 and RFC 9110 section 7.2.
    let mut head = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
        outgoing.method.name(),
        target.request_target,
        target.authority
    );
    if let Some(cookie) = cookie {
        head.push_str("Cookie: ");
        head.push_str(cookie);
        head.push_str("\r\n");
    }
    if let Some(media_type) = outgoing.media_type {
        head.push_str("Content-Type: ");
        head.push_str(media_type);
        head.push_str("\r\n");
    }
    // A POST sends Content-Length even for no body, a GET never: RFC 9110 section 8.6, and the
    // Fetch Standard, "HTTP-network-or-cache fetch", steps 5 to 9.
    if outgoing.method == Method::Post {
        let length = outgoing.body.map_or(0, <[u8]>::len);
        head.push_str(&format!("Content-Length: {length}\r\n"));
    }
    head.push_str("\r\n");
    // The body is written from the caller's slice, after the head: no copy of it is made.
    let body = outgoing.body.unwrap_or(&[]);
    stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.write_all(body))
        .and_then(|()| stream.flush())
        .map_err(|e| {
            tls_io_failure(&e).unwrap_or_else(|| match e.kind() {
                ErrorKind::WouldBlock | ErrorKind::TimedOut => unreachable(TIMED_OUT),
                _ => unreachable(format!("sending the request failed: {e}")),
            })
        })?;
    let mut wire = Wire::new(stream, limits.max_header_bytes.max(READ_CHUNK_BYTES));
    let head = read_final_head(&mut wire, limits)?;
    Ok((head, wire))
}

/// The `Location` of `head` when it is a redirect to follow: a redirect status (301, 302, 303,
/// 307 or 308) with a `Location` field. More than one `Location` field is
/// [`Failure::MalformedResponse`]: the Fetch Standard, "extract header list values".
fn redirect_location(head: &Head) -> Result<Option<&[u8]>, Failure> {
    if !matches!(head.status, 301 | 302 | 303 | 307 | 308) {
        return Ok(None);
    }
    let mut locations = head.values(b"location");
    let Some(location) = locations.next() else {
        return Ok(None);
    };
    if locations.next().is_some() {
        return Err(malformed("the redirect has more than one Location field"));
    }
    Ok(Some(location))
}

/// Resolve `location` against `base`, the current URL without the fragment it carried over,
/// whose fragment is `fragment`: the Fetch Standard, "location URL". Returns the resolved URL,
/// its serialization with the fragment carried over, and the fragment of that serialization.
fn resolve_location(
    location: &[u8],
    base: &surf_url::Url,
    fragment: Option<String>,
) -> Result<(surf_url::Url, String, Option<String>), Failure> {
    let location = std::str::from_utf8(location)
        .map_err(|_| malformed("the Location of the redirect is not UTF-8"))?;
    let resolved = surf_url::Url::parse(location, Some(base))
        .map_err(|e| malformed(format!("the Location of the redirect is {e}")))?;
    let mut href = resolved.href();
    // A URL without a fragment takes the fragment of the current URL. Both are serialized
    // fragments, so appending one is setting it.
    let fragment = match resolved.fragment() {
        Some(own) => Some(own.to_owned()),
        None => {
            if let Some(fragment) = &fragment {
                href.push('#');
                href.push_str(fragment);
            }
            fragment
        }
    };
    if href.len() > MAX_ADDRESS_BYTES {
        return Err(malformed(format!(
            "the Location of the redirect resolves to {} bytes, more than {MAX_ADDRESS_BYTES}",
            href.len()
        )));
    }
    // The response URL holds no more than its bytes: the crate documentation, "Memory".
    href.shrink_to_fit();
    Ok((resolved, href, fragment))
}

/// The detail of a fetch whose read timeout ran out after the connection was made.
const TIMED_OUT: &str = "the server did not complete the exchange within the read timeout";

/// The time from now until `deadline`, or `fallback` when there is no deadline.
fn time_left(deadline: Option<Instant>, fallback: Duration) -> Duration {
    deadline.map_or(fallback, |deadline| {
        deadline.saturating_duration_since(Instant::now())
    })
}

/// A connection whose writes and reads share one deadline: each waits at most the time left.
struct Timed {
    stream: TcpStream,
    deadline: Option<Instant>,
    timeout: Duration,
}

impl Timed {
    fn new(stream: TcpStream, timeout: Duration) -> Self {
        Timed {
            stream,
            deadline: Instant::now().checked_add(timeout),
            timeout,
        }
    }

    /// The time left, or a `TimedOut` error when there is none.
    fn left(&self) -> std::io::Result<Duration> {
        let left = time_left(self.deadline, self.timeout);
        if left.is_zero() {
            Err(std::io::Error::from(ErrorKind::TimedOut))
        } else {
            Ok(left)
        }
    }
}

impl Read for Timed {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let left = self.left()?;
        self.stream.set_read_timeout(Some(left))?;
        self.stream.read(buf)
    }
}

impl Write for Timed {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let left = self.left()?;
        self.stream.set_write_timeout(Some(left))?;
        self.stream.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

/// The connection of one request: `http:` over the socket, `https:` over a TLS connection on
/// it. Both share the deadline of the [`Timed`] beneath.
enum Stream {
    Plain(Timed),
    Tls(Box<Connection<Timed>>),
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Stream::Plain(timed) => timed.read(buf),
            Stream::Tls(connection) => connection.read(buf),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Stream::Plain(timed) => timed.write(buf),
            Stream::Tls(connection) => connection.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Stream::Plain(timed) => timed.flush(),
            Stream::Tls(connection) => connection.flush(),
        }
    }
}

/// The detail of a fetch refused because its set of trust anchors is empty.
const NO_ANCHORS: &str =
    "NoPathToAnchor: the set of trust anchors is empty, so no certificate can be trusted";

/// The verifier of a fetch's `https:` requests: path validation over `anchors`, or over the
/// bundled list when the request names none. An empty set is refused here, before any
/// connection (`surf_tls::anchors`: the place that builds a verifier refuses a count of 0).
fn path_verifier(anchors: Option<&Anchors>) -> Result<PathVerifier, Failure> {
    let bundled;
    let anchors = match anchors {
        Some(anchors) => anchors,
        None => {
            bundled = Anchors::bundled();
            &bundled
        }
    };
    if anchors.count() == 0 {
        return Err(Failure::CertificateNotTrusted(NO_ANCHORS.to_string()));
    }
    Ok(PathVerifier::new(anchors.as_slice().to_vec()))
}

/// The system clock as the time certificates are checked at (`docs/design/tls.md`, item 13). A
/// clock before 1970 reads as 1970, and every certificate is then not yet valid.
fn now() -> Time {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    Time::from_unix(i64::try_from(seconds).unwrap_or(i64::MAX))
}

/// The TLS handshake for `host` over `timed`, the server's certificates checked by `verifier`
/// at the system clock: the connection, or the failure of `docs/design/tls.md` item 7.
fn secure(timed: Timed, host: &str, verifier: &PathVerifier) -> Result<Stream, Failure> {
    match Connection::new(timed, host, verifier, now()) {
        Ok(connection) => Ok(Stream::Tls(Box::new(connection))),
        Err(error) => Err(tls_failure(&error)),
    }
}

/// The failure of a TLS error, as `docs/design/tls.md` item 7 maps it, with the error's own text
/// as the detail (a value from the server only as `surf_tls::error::excerpt` shows it). A
/// connection that ends, a socket's error and the read timeout are [`Failure::Unreachable`], as
/// for `http:`.
fn tls_failure(error: &TlsError) -> Failure {
    let detail = error.to_string();
    match error {
        TlsError::Certificate(CertificateRejection::NotTrusted(_)) => {
            Failure::CertificateNotTrusted(detail)
        }
        TlsError::Certificate(CertificateRejection::Expired { .. }) => {
            Failure::CertificateExpired(detail)
        }
        TlsError::Certificate(CertificateRejection::NameMismatch { .. }) | TlsError::Host(_) => {
            Failure::CertificateNameMismatch(detail)
        }
        TlsError::VersionUnsupported { .. }
        | TlsError::AlertReceived {
            description: AlertDescription::PROTOCOL_VERSION,
            before_server_hello: true,
        } => Failure::TlsVersionUnsupported(detail),
        TlsError::AlertReceived { .. }
        | TlsError::AlertSent { .. }
        | TlsError::SequenceExhausted
        | TlsError::KeyLimitReached => Failure::TlsHandshakeFailed(detail),
        TlsError::TimedOut => unreachable(TIMED_OUT),
        TlsError::ConnectionClosed | TlsError::Truncated | TlsError::Io(_) => {
            unreachable(format!("the TLS connection ended: {detail}"))
        }
    }
}

/// The failure of an I/O error that carries a TLS error ([`tls_failure`]); `None` for any other.
fn tls_io_failure(error: &std::io::Error) -> Option<Failure> {
    error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<TlsError>())
        .map(tls_failure)
}

fn unreachable(detail: impl Into<String>) -> Failure {
    Failure::Unreachable(detail.into())
}

fn malformed(detail: impl Into<String>) -> Failure {
    Failure::MalformedResponse(detail.into())
}

/// The scheme of an address this crate fetches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scheme {
    /// `http:`, port 80 by default.
    Http,
    /// `https:`, port 443 by default: HTTP/1.1 over TLS 1.3.
    Https,
}

/// The address without its scheme when it starts with `scheme` (which ends in `:`), in any
/// case.
fn after_scheme<'a>(address: &'a str, scheme: &str) -> Option<&'a str> {
    let head = address.get(..scheme.len())?;
    head.eq_ignore_ascii_case(scheme)
        .then(|| address.get(scheme.len()..).unwrap_or(""))
}

/// An `http:` or `https:` address split into what the request needs.
#[derive(Debug, PartialEq, Eq)]
struct Target {
    scheme: Scheme,
    /// The host to resolve, without the brackets of an IPv6 literal.
    host: String,
    port: u16,
    /// The authority as written in the address, sent as `Host`.
    authority: String,
    /// Path and query, never empty.
    request_target: String,
}

impl Target {
    fn parse(address: &str) -> Result<Target, Failure> {
        let (scheme, rest) = if let Some(rest) = after_scheme(address, "http:") {
            (Scheme::Http, rest)
        } else if let Some(rest) = after_scheme(address, "https:") {
            (Scheme::Https, rest)
        } else {
            return Err(Failure::UnsupportedScheme(match address.split_once(':') {
                Some((scheme, _)) if scheme.len() <= 32 => {
                    format!("the scheme {scheme:?} is not fetched; only http and https are")
                }
                _ => "the address does not start with the http: or https: scheme".to_string(),
            }));
        };
        if let Some(byte) = rest.bytes().find(|b| !(0x21..=0x7e).contains(b)) {
            return Err(unreachable(format!(
                "the address holds the byte 0x{byte:02x}, which is not printable ASCII"
            )));
        }
        let Some(rest) = rest.strip_prefix("//") else {
            return Err(unreachable(
                "the address is not of the form http://host/path or https://host/path",
            ));
        };
        let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let (authority, path_and_more) = rest.split_at_checked(authority_end).unwrap_or((rest, ""));
        // User information is treated as an error: RFC 9110 section 4.2.4.
        if authority.contains('@') {
            return Err(unreachable(
                "the address carries user information, which is refused",
            ));
        }
        let (host, port) = if let Some(bracketed) = authority.strip_prefix('[') {
            let Some((host, after)) = bracketed.split_once(']') else {
                return Err(unreachable("the address has an unclosed [ in its host"));
            };
            // Only an IPv6 address stands in brackets here: RFC 3986 section 3.2.2 allows
            // IPv6 or IPvFuture, and no IPvFuture form is used.
            if host.parse::<Ipv6Addr>().is_err() {
                return Err(unreachable(
                    "the address has a host in brackets that is not an IPv6 address",
                ));
            }
            match after {
                "" => (host, None),
                _ => match after.strip_prefix(':') {
                    Some(port) => (host, Some(port)),
                    None => return Err(unreachable("the address has text after its ] host")),
                },
            }
        } else {
            match authority.split_once(':') {
                Some((host, port)) => (host, Some(port)),
                None => (authority, None),
            }
        };
        // An empty host is invalid: RFC 9110 section 4.2.1.
        if host.is_empty() {
            return Err(unreachable("the address has an empty host"));
        }
        // An empty or absent port is 80 for http and 443 for https: RFC 9110 sections 4.2.1
        // and 4.2.2.
        let port = match port {
            None | Some("") => match scheme {
                Scheme::Http => 80,
                Scheme::Https => 443,
            },
            Some(text) => match text.parse::<u16>() {
                Ok(port) if port != 0 && text.bytes().all(|b| b.is_ascii_digit()) => port,
                _ => return Err(unreachable(format!("the address has the port {text:?}"))),
            },
        };
        let path_and_query = path_and_more.split('#').next().unwrap_or("");
        let request_target = if path_and_query.starts_with('/') {
            path_and_query.to_string()
        } else {
            // An empty path is sent as "/": RFC 9112 section 3.2.1.
            format!("/{path_and_query}")
        };
        Ok(Target {
            scheme,
            host: host.to_string(),
            port,
            authority: authority.to_string(),
            request_target,
        })
    }
}

/// Why a request must go to public addresses only (the crate documentation, "Destinations").
#[derive(Clone, Copy)]
enum Refusing {
    /// It follows a redirect, and the first request went to a public address.
    Redirect,
    /// It is the first request, and it comes from a page at a public address.
    FromPublicPage,
}

/// Resolve once, check, and connect to one of the addresses checked, all within `timeout`.
/// With `public_only`, any local address among those resolved refuses the request before a
/// connection (the crate documentation, "Destinations"). Returns the connection and the address
/// it went to.
fn connect(
    target: &Target,
    timeout: Duration,
    network: &dyn Network,
    public_only: Option<Refusing>,
) -> Result<(TcpStream, SocketAddr), Failure> {
    let deadline = Instant::now().checked_add(timeout);
    let addresses = match target.host.parse::<IpAddr>() {
        Ok(ip) => vec![SocketAddr::new(ip, target.port)],
        Err(_) => network.resolve(&target.host, target.port, timeout)?,
    };
    let refused = public_only.and_then(|refusing| {
        addresses
            .iter()
            .find(|address| is_local(address.ip()))
            .map(|local| (refusing, local.ip()))
    });
    if let Some((refusing, ip)) = refused {
        let host = &target.host;
        return Err(Failure::ForbiddenDestination(match refusing {
            Refusing::Redirect => format!(
                "a redirect leads to {host}, which is the local address {ip}, and the first \
                 request went to a public address"
            ),
            Refusing::FromPublicPage => format!(
                "{host} is the local address {ip}, and the request comes from a page at a \
                 public address"
            ),
        }));
    }
    let mut last_error = None;
    for address in &addresses {
        let left = time_left(deadline, timeout);
        if left.is_zero() {
            last_error = Some(format!(
                "the connect timeout ran out before connecting to {address}"
            ));
            break;
        }
        match network.connect(address, left) {
            Ok(stream) => return Ok((stream, *address)),
            Err(e) => last_error = Some(format!("connecting to {address} failed: {e}")),
        }
    }
    Err(unreachable(last_error.unwrap_or_else(|| {
        format!("{} resolves to no address", target.host)
    })))
}

/// Whether `ip` is a local address in the sense of the crate documentation, "Destinations":
/// loopback, private, link-local, unique-local, shared or unspecified, or an IPv6 address that
/// embeds one of these (IPv4-mapped, IPv4-compatible, NAT64 or 6to4). The rule a redirect's
/// destination is checked by, and
/// the one `surf-session` applies to the target of `follow` when it is an IP address.
pub fn is_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_local_v4(v4),
        IpAddr::V6(v6) => {
            let first = v6.segments()[0];
            embedded_v4(v6).is_some_and(is_local_v4)
                || v6.is_loopback()
                || v6.is_unspecified()
                // fe80::/10, link-local
                || first & 0xffc0 == 0xfe80
                // fc00::/7, unique-local
                || first & 0xfe00 == 0xfc00
        }
    }
}

/// The IPv4 address an IPv6 address embeds: IPv4-mapped (`::ffff:a.b.c.d`), IPv4-compatible
/// (`::a.b.c.d`) and the NAT64 well-known prefix (`64:ff9b::/96`) in their last 32 bits, 6to4
/// (`2002::/16`) in the 32 bits after its prefix.
fn embedded_v4(ip: Ipv6Addr) -> Option<Ipv4Addr> {
    let joined = |high: u16, low: u16| Ipv4Addr::from((u32::from(high) << 16) | u32::from(low));
    match ip.segments() {
        [0, 0, 0, 0, 0, 0xffff, high, low]
        | [0, 0, 0, 0, 0, 0, high, low]
        | [0x64, 0xff9b, 0, 0, 0, 0, high, low]
        | [0x2002, high, low, ..] => Some(joined(high, low)),
        _ => None,
    }
}

fn is_local_v4(ip: Ipv4Addr) -> bool {
    let [a, b, _, _] = ip.octets();
    a == 127
        || a == 10
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 168)
        || (a == 169 && b == 254)
        || (a == 100 && (64..=127).contains(&b))
        || ip.is_unspecified()
}

/// The addresses of the name `host` with `port`, resolved on a thread of its own, waited for at
/// most `timeout`. A resolution that takes longer is abandoned: its thread ends when the
/// resolver returns, and its answer is dropped.
fn resolve_name(host: &str, port: u16, timeout: Duration) -> Result<Vec<SocketAddr>, Failure> {
    let (sender, receiver) = mpsc::channel();
    let owned = host.to_string();
    thread::Builder::new()
        .name("surf-fetch-resolve".to_string())
        .spawn(move || {
            let addresses = (owned.as_str(), port)
                .to_socket_addrs()
                .map(Iterator::collect::<Vec<SocketAddr>>);
            // The receiver is gone when the fetch stopped waiting; the answer is not needed.
            let _ = sender.send(addresses);
        })
        .map_err(|e| unreachable(format!("{host} cannot be resolved: {e}")))?;
    match receiver.recv_timeout(timeout) {
        Ok(Ok(addresses)) => Ok(addresses),
        Ok(Err(e)) => Err(unreachable(format!("{host} does not resolve: {e}"))),
        Err(RecvTimeoutError::Timeout) => Err(unreachable(format!(
            "{host} did not resolve within the connect timeout"
        ))),
        Err(RecvTimeoutError::Disconnected) => Err(unreachable(format!(
            "resolving {host} ended without an answer"
        ))),
    }
}

/// The outcome of reading one line.
enum LineRead {
    /// The line without its terminator, and the bytes it took, terminator included.
    Line(Vec<u8>, usize),
    /// No line terminator within the budget.
    TooLong,
    /// The connection ended before a line terminator.
    Closed,
}

/// Make room in `buf` for `additional` more bytes. The capacity doubles, as a [`Vec`]'s does,
/// but not past `ceiling`, unless the bytes themselves need more.
fn reserve_within(buf: &mut Vec<u8>, additional: usize, ceiling: usize) {
    let needed = buf.len().saturating_add(additional);
    if needed <= buf.capacity() {
        return;
    }
    let target = buf
        .capacity()
        .saturating_mul(2)
        .max(READ_CHUNK_BYTES)
        .min(ceiling)
        .max(needed);
    buf.reserve_exact(target - buf.len());
}

/// A connection with the bytes read from it and not yet consumed.
struct Wire<R> {
    inner: R,
    buf: Vec<u8>,
    start: usize,
    /// The capacity `buf` doubles up to; past it, it grows by what a read needs only.
    ceiling: usize,
}

impl<R: Read> Wire<R> {
    fn new(inner: R, ceiling: usize) -> Self {
        Wire {
            inner,
            buf: Vec::new(),
            start: 0,
            ceiling,
        }
    }

    fn available(&self) -> &[u8] {
        self.buf.get(self.start..).unwrap_or(&[])
    }

    fn consume(&mut self, n: usize) {
        self.start = self.start.saturating_add(n).min(self.buf.len());
    }

    /// Drop the bytes consumed and keep at most one read buffer of capacity, for the body: a
    /// body is read at most one read buffer at a time, and what the head left unconsumed came
    /// in the read that ended it.
    fn release(&mut self) {
        self.buf.drain(..self.start);
        self.start = 0;
        self.ceiling = READ_CHUNK_BYTES;
        self.buf.shrink_to(READ_CHUNK_BYTES);
    }

    /// Read once from the connection, adding between 1 and `max` bytes (at most one read
    /// buffer). Returns 0 when the connection has ended.
    fn fill(&mut self, max: usize) -> Result<usize, Failure> {
        if self.start > 0 {
            self.buf.drain(..self.start);
            self.start = 0;
        }
        let mut chunk = [0u8; READ_CHUNK_BYTES];
        let want = max.clamp(1, READ_CHUNK_BYTES);
        let window = chunk.get_mut(..want).unwrap_or(&mut []);
        loop {
            match self.inner.read(window) {
                Ok(n) => {
                    let got = window.get(..n).unwrap_or(&*window);
                    reserve_within(&mut self.buf, got.len(), self.ceiling);
                    self.buf.extend_from_slice(got);
                    return Ok(got.len());
                }
                // A TLS error first: the TLS connection reports its own failures, a stream that
                // ended without close_notify among them, inside an I/O error.
                Err(e) => match tls_io_failure(&e) {
                    Some(failure) => return Err(failure),
                    None if e.kind() == ErrorKind::Interrupted => continue,
                    None if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                        return Err(unreachable(TIMED_OUT))
                    }
                    None => return Err(unreachable(format!("reading the response failed: {e}"))),
                },
            }
        }
    }

    /// Read one line ended by LF, an optional CR before it removed, taking at most `budget`
    /// bytes. A single LF is accepted as a line terminator: RFC 9112 section 2.2.
    fn read_line(&mut self, budget: usize) -> Result<LineRead, Failure> {
        let mut scanned = 0;
        loop {
            let available = self.available();
            let window = available.len().min(budget);
            let found = available
                .get(scanned..window)
                .and_then(|w| w.iter().position(|&b| b == b'\n'));
            if let Some(offset) = found {
                let end = scanned.saturating_add(offset);
                let mut line = available.get(..end).unwrap_or(&[]).to_vec();
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                let used = end.saturating_add(1);
                self.consume(used);
                return Ok(LineRead::Line(line, used));
            }
            if available.len() >= budget {
                return Ok(LineRead::TooLong);
            }
            let room = budget - available.len();
            scanned = window;
            if self.fill(room)? == 0 {
                return Ok(LineRead::Closed);
            }
        }
    }

    /// Move exactly `n` bytes into `out`, growing it past `ceiling` only as far as the bytes
    /// need.
    fn read_exact_into(
        &mut self,
        out: &mut Vec<u8>,
        n: usize,
        ceiling: usize,
        what: &str,
    ) -> Result<(), Failure> {
        let mut left = n;
        while left > 0 {
            if self.available().is_empty() && self.fill(left)? == 0 {
                return Err(unreachable(format!(
                    "the connection ended {left} bytes before the end of the {what}"
                )));
            }
            let available = self.available();
            let take = left.min(available.len());
            reserve_within(out, take, ceiling);
            out.extend_from_slice(available.get(..take).unwrap_or(&[]));
            self.consume(take);
            left -= take;
        }
        Ok(())
    }
}

/// A status line and its header fields, names in lower case.
struct Head {
    minor_version: u8,
    status: u16,
    fields: Vec<(Vec<u8>, Vec<u8>)>,
}

impl Head {
    fn values<'a>(&'a self, name: &'a [u8]) -> impl Iterator<Item = &'a [u8]> + 'a {
        self.fields
            .iter()
            .filter(move |(n, _)| n.as_slice() == name)
            .map(|(_, v)| v.as_slice())
    }
}

/// Read a whole response from `inner`, with an empty `url`: the parsing [`fetch`] does after
/// the request, without a connection.
#[cfg(test)]
fn read_response<R: Read>(inner: R, limits: &Limits) -> Result<Response, Failure> {
    let mut wire = Wire::new(inner, limits.max_header_bytes.max(READ_CHUNK_BYTES));
    let head = read_final_head(&mut wire, limits)?;
    let (status, media_type, body) = read_rest(wire, head, limits.max_body_bytes)?;
    Ok(Response {
        status,
        media_type,
        body,
        url: String::new(),
        method: Method::Get,
    })
}

/// The status, the media type and the body of the response whose head is `head`. What the body
/// needs of the head is taken first; the head is dropped and the connection's buffer released
/// before a body byte is read (the crate documentation, "Memory").
fn read_rest<R: Read>(
    mut wire: Wire<R>,
    head: Head,
    max: usize,
) -> Result<(u16, String, Vec<u8>), Failure> {
    let status = head.status;
    let media_type = media_type(&head);
    let framing = framing(&head, max)?;
    drop(head);
    wire.release();
    let body = read_body(&mut wire, framing, max)?;
    Ok((status, media_type, body))
}

/// Read heads until one is not interim (1xx), within one header budget.
fn read_final_head<R: Read>(wire: &mut Wire<R>, limits: &Limits) -> Result<Head, Failure> {
    let mut budget = limits.max_header_bytes;
    let head = loop {
        let head = read_head(wire, &mut budget, limits.max_header_bytes)?;
        match head.status {
            // 101 switches to a protocol this crate never asked for.
            101 => {
                return Err(malformed(
                    "101 Switching Protocols without an upgrade request",
                ))
            }
            // Interim responses are parsed and ignored: RFC 9110 section 15.2.
            100..=199 => continue,
            _ => break head,
        }
    };
    Ok(head)
}

fn header_limit(max: usize) -> Failure {
    malformed(format!("the header section is larger than {max} bytes"))
}

fn read_head<R: Read>(wire: &mut Wire<R>, budget: &mut usize, max: usize) -> Result<Head, Failure> {
    expect_http_name(wire)?;
    let (line, used) = match wire.read_line(*budget)? {
        LineRead::Line(line, used) => (line, used),
        LineRead::TooLong => return Err(header_limit(max)),
        LineRead::Closed => return Err(unreachable("the connection ended inside the status line")),
    };
    *budget = budget.saturating_sub(used);
    let (minor_version, status) = parse_status_line(&line)?;
    let mut fields: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    loop {
        let (line, used) = match wire.read_line(*budget)? {
            LineRead::Line(line, used) => (line, used),
            LineRead::TooLong => return Err(header_limit(max)),
            LineRead::Closed => {
                return Err(unreachable(
                    "the connection ended inside the header section",
                ))
            }
        };
        *budget = budget.saturating_sub(used);
        let Some(&first) = line.first() else {
            break;
        };
        // A bare CR makes the element invalid: RFC 9112 section 2.2.
        if line.contains(&b'\r') {
            return Err(malformed("a header field line holds a bare CR"));
        }
        if first == b' ' || first == b'\t' {
            // Obsolete line folding is replaced by SP: RFC 9112 section 5.2. Whitespace before
            // the first field line is rejected: RFC 9112 section 2.2.
            let Some((_, value)) = fields.last_mut() else {
                return Err(malformed(
                    "whitespace between the status line and the first field",
                ));
            };
            value.push(b' ');
            value.extend_from_slice(trim_ows(&line));
            continue;
        }
        let Some(colon) = line.iter().position(|&b| b == b':') else {
            return Err(malformed("a header field line has no colon"));
        };
        let (name, rest) = line.split_at(colon);
        // Field names are tokens, so whitespace before the colon is invalid: RFC 9110 section
        // 5.6.2 and RFC 9112 section 5.1.
        if name.is_empty() || !name.iter().all(|&b| is_tchar(b)) {
            return Err(malformed("a header field name is not a token"));
        }
        let value = trim_ows(rest.get(1..).unwrap_or(&[]));
        fields.push((name.to_ascii_lowercase(), value.to_vec()));
    }
    Ok(Head {
        minor_version,
        status,
        fields,
    })
}

/// Fail as soon as the first bytes cannot begin `HTTP/`, so a server that speaks another
/// protocol is told apart from one that is slow.
fn expect_http_name<R: Read>(wire: &mut Wire<R>) -> Result<(), Failure> {
    const NAME: &[u8] = b"HTTP/";
    loop {
        let available = wire.available();
        let have = available.len().min(NAME.len());
        if available.get(..have) != NAME.get(..have) {
            return Err(malformed("the reply does not start with HTTP/"));
        }
        if have == NAME.len() {
            return Ok(());
        }
        if wire.fill(NAME.len() - have)? == 0 {
            return Err(unreachable(if have == 0 {
                "the connection ended before any response"
            } else {
                "the connection ended inside the status line"
            }));
        }
    }
}

/// Parse `HTTP/1.x SP 3DIGIT [SP reason-phrase]` into the minor version and the status code:
/// RFC 9112 sections 2.3 and 4. A missing SP after the code is accepted.
fn parse_status_line(line: &[u8]) -> Result<(u8, u16), Failure> {
    if line.contains(&b'\r') {
        return Err(malformed("the status line holds a bare CR"));
    }
    let (minor, digits) = match line {
        [b'H', b'T', b'T', b'P', b'/', b'1', b'.', minor, b' ', a, b, c, rest @ ..]
            if minor.is_ascii_digit()
                && [a, b, c].iter().all(|d| d.is_ascii_digit())
                && matches!(rest.first(), None | Some(b' ')) =>
        {
            (minor - b'0', [a - b'0', b - b'0', c - b'0'])
        }
        [b'H', b'T', b'T', b'P', b'/', major, b'.', minor, ..]
            if major.is_ascii_digit() && minor.is_ascii_digit() && *major != b'1' =>
        {
            return Err(malformed("the response is not HTTP/1.x"));
        }
        _ => return Err(malformed("the status line is not HTTP/1.x SP 3DIGIT")),
    };
    let status = u16::from(digits[0]) * 100 + u16::from(digits[1]) * 10 + u16::from(digits[2]);
    // Values outside 100..599 are invalid: RFC 9110 section 15.
    if !(100..=599).contains(&status) {
        return Err(malformed(format!(
            "the status code {status} is outside 100..599"
        )));
    }
    Ok((minor, status))
}

/// How the body of a response ends, read from its head.
enum Framing {
    /// No body.
    None,
    /// The chunked coding.
    Chunked,
    /// A `Content-Length` of at most the body limit.
    Length(usize),
    /// When the server closes the connection.
    UntilClose,
}

/// The framing of the body by the rules of RFC 9112 section 6.3, for the response to a `GET` or
/// a `POST` (rule 2 is for `HEAD` and `CONNECT`, never sent).
/// A framing this crate refuses, and a declared length over `max`, fail here, before any body
/// byte is read.
fn framing(head: &Head, max: usize) -> Result<Framing, Failure> {
    // 204 and 304 never have a body (rule 1; 1xx was skipped and HEAD is never sent).
    if head.status == 204 || head.status == 304 {
        return Ok(Framing::None);
    }
    let mut transfer_codings = head
        .values(b"transfer-encoding")
        .flat_map(|v| v.split(|&b| b == b','))
        .map(trim_ows)
        .filter(|c| !c.is_empty())
        .peekable();
    if head.values(b"transfer-encoding").next().is_some() {
        // An HTTP/1.0 message with Transfer-Encoding has faulty framing: RFC 9112 section 6.1.
        if head.minor_version == 0 {
            return Err(malformed("an HTTP/1.0 response carries Transfer-Encoding"));
        }
        // Transfer-Encoding overrides Content-Length (rule 3). Only chunked alone is decoded;
        // any other coding would leave the body encoded, so it is refused (rule 4).
        let only_chunked = transfer_codings
            .next()
            .is_some_and(|c| c.eq_ignore_ascii_case(b"chunked"))
            && transfer_codings.peek().is_none();
        if !only_chunked {
            return Err(malformed("a transfer coding other than chunked alone"));
        }
        return Ok(Framing::Chunked);
    }
    if head.values(b"content-length").next().is_some() {
        let length = content_length(head)?;
        // Fail before reading: the limit is checked against the declared length.
        if length > max as u64 {
            return Err(body_limit(max));
        }
        let length = usize::try_from(length).map_err(|_| body_limit(max))?;
        return Ok(Framing::Length(length));
    }
    // No declared length: the body ends when the server closes the connection (rule 8).
    Ok(Framing::UntilClose)
}

/// Read the body as `framing` says it ends, within the body limit `max`.
fn read_body<R: Read>(
    wire: &mut Wire<R>,
    framing: Framing,
    max: usize,
) -> Result<Vec<u8>, Failure> {
    match framing {
        Framing::None => Ok(Vec::new()),
        Framing::Chunked => read_chunked(wire, max),
        Framing::Length(length) => {
            let mut body = Vec::with_capacity(length.min(READ_CHUNK_BYTES));
            wire.read_exact_into(&mut body, length, length, "body")?;
            // Bytes after the declared length are discarded: RFC 9112 section 6.3.
            Ok(body)
        }
        Framing::UntilClose => read_until_close(wire, max),
    }
}

/// Read a body that ends when the server closes the connection. It takes at most one byte more
/// than the limit `max`, which is enough to tell it is too long.
fn read_until_close<R: Read>(wire: &mut Wire<R>, max: usize) -> Result<Vec<u8>, Failure> {
    let ceiling = max.saturating_add(1);
    let mut body = Vec::new();
    loop {
        let available = wire.available();
        let taken = available.len().min(ceiling.saturating_sub(body.len()));
        reserve_within(&mut body, taken, ceiling);
        body.extend_from_slice(available.get(..taken).unwrap_or(&[]));
        wire.consume(taken);
        if body.len() > max {
            return Err(body_limit(max));
        }
        if wire.fill(ceiling.saturating_sub(body.len()))? == 0 {
            return Ok(body);
        }
    }
}

fn body_limit(max: usize) -> Failure {
    malformed(format!("the body is larger than {max} bytes"))
}

/// The value of `Content-Length`: every field and every list member must be the same decimal
/// number, else the framing is invalid (RFC 9112 section 6.3, rule 5). Overflow is invalid.
fn content_length(head: &Head) -> Result<u64, Failure> {
    let mut found: Option<u64> = None;
    for member in head
        .values(b"content-length")
        .flat_map(|v| v.split(|&b| b == b','))
        .map(trim_ows)
        .filter(|m| !m.is_empty())
    {
        let value = parse_decimal(member)
            .ok_or_else(|| malformed("Content-Length is not a decimal number of bytes"))?;
        match found {
            Some(previous) if previous != value => {
                return Err(malformed("Content-Length has differing values"))
            }
            _ => found = Some(value),
        }
    }
    found.ok_or_else(|| malformed("Content-Length is empty"))
}

fn parse_decimal(text: &[u8]) -> Option<u64> {
    if text.is_empty() {
        return None;
    }
    text.iter().try_fold(0u64, |acc, &b| {
        let digit = b.is_ascii_digit().then(|| u64::from(b - b'0'))?;
        acc.checked_mul(10)?.checked_add(digit)
    })
}

/// Remove the chunked coding: RFC 9112 section 7.1. The body is complete at the zero-size
/// chunk (RFC 9112 section 8); the trailer section after it is not read.
fn read_chunked<R: Read>(wire: &mut Wire<R>, max: usize) -> Result<Vec<u8>, Failure> {
    let mut body = Vec::new();
    loop {
        let line = match wire.read_line(MAX_CHUNK_LINE_BYTES)? {
            LineRead::Line(line, _) => line,
            LineRead::TooLong => {
                return Err(malformed(format!(
                    "a chunk-size line is longer than {MAX_CHUNK_LINE_BYTES} bytes"
                )))
            }
            LineRead::Closed => {
                return Err(unreachable("the connection ended inside the chunked body"))
            }
        };
        let size = parse_chunk_size(&line)?;
        if size == 0 {
            return Ok(body);
        }
        let room = max.saturating_sub(body.len());
        if size > room as u64 {
            return Err(body_limit(max));
        }
        let size = usize::try_from(size).map_err(|_| body_limit(max))?;
        wire.read_exact_into(&mut body, size, max, "chunk")?;
        match wire.read_line(2)? {
            LineRead::Line(line, _) if line.is_empty() => {}
            LineRead::Line(..) | LineRead::TooLong => {
                return Err(malformed("chunk data is not followed by CRLF"))
            }
            LineRead::Closed => {
                return Err(unreachable("the connection ended inside the chunked body"))
            }
        }
    }
}

/// Parse `chunk-size [chunk-ext]`; extensions are ignored (RFC 9112 section 7.1.1) and the
/// size is parsed with checked arithmetic (RFC 9112 section 7.1).
fn parse_chunk_size(line: &[u8]) -> Result<u64, Failure> {
    if line.contains(&b'\r') {
        return Err(malformed("a chunk-size line holds a bare CR"));
    }
    if line.first().is_some_and(|&b| b == b' ' || b == b'\t') {
        return Err(malformed("a chunk-size line starts with whitespace"));
    }
    // Whitespace may stand between the size and a ";" (BWS in chunk-ext).
    let size = trim_ows(line.split(|&b| b == b';').next().unwrap_or(&[]));
    if size.is_empty() {
        return Err(malformed("a chunk-size line has no size"));
    }
    size.iter()
        .try_fold(0u64, |acc, &b| {
            let digit = char::from(b).to_digit(16)?;
            acc.checked_mul(16)?.checked_add(u64::from(digit))
        })
        .ok_or_else(|| malformed("a chunk size is not a hexadecimal number that fits 64 bits"))
}

/// The media type of `Content-Type` (RFC 9110 section 8.3.1): `type/subtype` in lower case,
/// parameters dropped. The last `Content-Type` field is used. A missing field, or one that is
/// not `token/token`, is `application/octet-stream` (RFC 9110 section 8.3).
fn media_type(head: &Head) -> String {
    let Some(value) = head.values(b"content-type").last() else {
        return DEFAULT_MEDIA_TYPE.to_string();
    };
    let essence = trim_ows(value.split(|&b| b == b';').next().unwrap_or(&[]));
    let mut parts = essence.splitn(2, |&b| b == b'/');
    match (parts.next(), parts.next()) {
        (Some(kind), Some(subtype)) if is_token(kind) && is_token(subtype) => {
            let mut text = String::with_capacity(essence.len());
            for &b in essence {
                text.push(char::from(b.to_ascii_lowercase()));
            }
            text
        }
        _ => DEFAULT_MEDIA_TYPE.to_string(),
    }
}

fn is_token(text: &[u8]) -> bool {
    !text.is_empty() && text.iter().all(|&b| is_tchar(b))
}

/// `tchar`: RFC 9110 section 5.6.2.
fn is_tchar(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}

/// Remove leading and trailing SP and HTAB (OWS): RFC 9110 section 5.6.3.
fn trim_ows(text: &[u8]) -> &[u8] {
    let is_ows = |b: &u8| *b == b' ' || *b == b'\t';
    let start = text.iter().position(|b| !is_ows(b)).unwrap_or(text.len());
    let end = text
        .iter()
        .rposition(|b| !is_ows(b))
        .map_or(start, |i| i + 1);
    text.get(start..end).unwrap_or(&[])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hands out its bytes `step` at a time, to show the result does not depend on how the
    /// network splits them.
    struct Trickle<'a> {
        bytes: &'a [u8],
        step: usize,
    }

    impl Read for Trickle<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let n = self.step.min(buf.len()).min(self.bytes.len());
            let (head, tail) = self.bytes.split_at(n);
            buf[..n].copy_from_slice(head);
            self.bytes = tail;
            Ok(n)
        }
    }

    fn small() -> Limits {
        Limits {
            max_header_bytes: 256,
            max_body_bytes: 512,
            ..Limits::default()
        }
    }

    fn parse(bytes: &[u8], limits: &Limits) -> Result<Response, Failure> {
        read_response(bytes, limits)
    }

    #[test]
    fn target_splits_host_port_and_request_target() {
        let t = Target::parse("http://Example.org:8080/a/b?c=d#e").expect("parses");
        assert_eq!(t.host, "Example.org");
        assert_eq!(t.port, 8080);
        assert_eq!(t.authority, "Example.org:8080");
        assert_eq!(t.request_target, "/a/b?c=d");
        let t = Target::parse("http://[::1]?q").expect("parses");
        assert_eq!((t.host.as_str(), t.port), ("::1", 80));
        assert_eq!(t.authority, "[::1]");
        assert_eq!(t.request_target, "/?q");
        let t = Target::parse("http://h:").expect("parses");
        assert_eq!(t.port, 80);
        let t = Target::parse("HTTPS://h/x").expect("parses");
        assert_eq!((t.scheme, t.port), (Scheme::Https, 443));
        assert_eq!(t.request_target, "/x");
        let t = Target::parse("https://h:8443").expect("parses");
        assert_eq!(
            (t.scheme, t.port, t.authority.as_str()),
            (Scheme::Https, 8443, "h:8443")
        );
    }

    #[test]
    fn target_refuses_what_it_cannot_use() {
        for address in [
            "http:h/x",
            "http://",
            "http://:80/",
            "http://h:+80/",
            "http://h:0/",
            "http://[::1/",
            "http://[::1]x/",
            "http://u@h/",
            "http://h/\u{e9}",
            "https:h/x",
            "https://u@h/",
        ] {
            assert!(
                matches!(Target::parse(address), Err(Failure::Unreachable(_))),
                "{address:?}"
            );
        }
        for address in [
            "",
            "htt",
            "httpss://h/",
            "ftp://h/",
            "\u{e9}ttp://h/",
            "mailto:a@b",
        ] {
            assert!(
                matches!(Target::parse(address), Err(Failure::UnsupportedScheme(_))),
                "{address:?}"
            );
        }
    }

    #[test]
    fn bare_lf_and_obs_fold_are_accepted() {
        let result = parse(
            b"HTTP/1.0 200 OK\nContent-Type: text/\n plain\nContent-Length: 2\n\nok",
            &small(),
        );
        // The folded value "text/ plain" is not a media type, so the default applies.
        assert_eq!(
            result,
            Ok(Response {
                status: 200,
                media_type: DEFAULT_MEDIA_TYPE.to_string(),
                body: b"ok".to_vec(),
                url: String::new(),
                method: Method::Get,
            })
        );
    }

    #[test]
    fn repeated_equal_content_lengths_are_one() {
        let result = parse(
            b"HTTP/1.1 200 OK\r\nContent-Length: 2, 2\r\nContent-Length: 2\r\n\r\nokEXTRA",
            &small(),
        );
        assert_eq!(result.map(|r| r.body), Ok(b"ok".to_vec()));
    }

    #[test]
    fn content_length_overflow_is_malformed() {
        let result = parse(
            b"HTTP/1.1 200 OK\r\nContent-Length: 99999999999999999999999\r\n\r\n",
            &small(),
        );
        assert!(
            matches!(result, Err(Failure::MalformedResponse(_))),
            "{result:?}"
        );
    }

    #[test]
    fn whitespace_before_colon_is_malformed() {
        let result = parse(b"HTTP/1.1 200 OK\r\nContent-Length : 0\r\n\r\n", &small());
        assert!(
            matches!(result, Err(Failure::MalformedResponse(_))),
            "{result:?}"
        );
    }

    #[test]
    fn long_chunk_line_is_malformed() {
        let mut reply = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;".to_vec();
        reply.extend(std::iter::repeat_n(b'x', 2 * MAX_CHUNK_LINE_BYTES));
        let result = parse(&reply, &small());
        assert!(
            matches!(result, Err(Failure::MalformedResponse(_))),
            "{result:?}"
        );
    }

    #[test]
    fn chunk_without_crlf_is_malformed() {
        let result = parse(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nokX\r\n0\r\n\r\n",
            &small(),
        );
        assert!(
            matches!(result, Err(Failure::MalformedResponse(_))),
            "{result:?}"
        );
    }

    #[test]
    fn transfer_encoding_in_http_1_0_is_malformed() {
        let result = parse(
            b"HTTP/1.0 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
            &small(),
        );
        assert!(
            matches!(result, Err(Failure::MalformedResponse(_))),
            "{result:?}"
        );
    }

    #[test]
    fn switching_protocols_is_malformed() {
        let result = parse(b"HTTP/1.1 101 Switching Protocols\r\n\r\n", &small());
        assert!(
            matches!(result, Err(Failure::MalformedResponse(_))),
            "{result:?}"
        );
    }

    #[test]
    fn header_budget_spans_interim_responses() {
        let mut reply = Vec::new();
        for _ in 0..20 {
            reply.extend_from_slice(b"HTTP/1.1 100 Continue\r\n\r\n");
        }
        reply.extend_from_slice(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
        let result = parse(&reply, &small());
        assert!(
            matches!(result, Err(Failure::MalformedResponse(_))),
            "{result:?}"
        );
    }

    /// The buffer of bytes read and not yet consumed doubles only up to its ceiling (the header
    /// limit), plus what one read needs: a header line of 40,000 bytes read in pieces of 4095
    /// would otherwise double the buffer to 65,520 bytes.
    #[test]
    fn the_read_buffer_grows_within_the_header_limit_plus_one_read() {
        let limit = 40_100;
        let mut bytes = vec![b'a'; 40_000];
        bytes.extend_from_slice(b"\r\n");
        let mut wire = Wire::new(
            Trickle {
                bytes: &bytes,
                step: 4095,
            },
            limit,
        );
        assert!(matches!(wire.read_line(limit), Ok(LineRead::Line(..))));
        assert!(
            wire.buf.capacity() <= limit + READ_CHUNK_BYTES,
            "header limit {limit} bytes, read buffer allocated {} bytes",
            wire.buf.capacity()
        );
    }

    /// Generated inputs: none panics, and every one gives the same result whether it arrives
    /// at once or one byte at a time.
    #[test]
    fn generated_inputs_never_panic_and_ignore_fragmentation() {
        const PIECES: &[&[u8]] = &[
            b"HTTP/1.1 ",
            b"HTTP/1.0 ",
            b"200",
            b"404 ",
            b"100 ",
            b"OK",
            b"\r\n",
            b"\n",
            b"\r",
            b" ",
            b":",
            b";",
            b",",
            b"Content-Length: ",
            b"Transfer-Encoding: chunked",
            b"Content-Type: text/HTML; q=1",
            b"0",
            b"5",
            b"FFFFFFFFFFFFFFFFFFFF",
            b"hello",
            b"\x00\xff",
        ];
        let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let limits = Limits {
            max_header_bytes: 128,
            max_body_bytes: 64,
            ..Limits::default()
        };
        const SEEDS: &[&[u8]] = &[
            b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 5\r\n\r\nhello",
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5;x=y\r\nhello\r\n0\r\n\r\n",
            b"HTTP/1.0 404 Not Found\r\nContent-Type: Text/Plain; a=b\r\n\r\nuntil close",
            b"HTTP/1.1 103 Early\r\nLink: x\r\n\r\nHTTP/1.1 204 No Content\r\n\r\n",
        ];
        let mut seen = [0usize; 3];
        for _ in 0..5000 {
            let pick = |n: u64, len: usize| (n % (len as u64).max(1)) as usize;
            let mut input = SEEDS[pick(next(), SEEDS.len())].to_vec();
            for _ in 0..next() % 4 {
                let at = pick(next(), input.len() + 1);
                match next() % 3 {
                    0 => input.truncate(at),
                    1 => {
                        if let Some(byte) = input.get_mut(at) {
                            *byte = next() as u8;
                        }
                    }
                    _ => {
                        let piece = PIECES[pick(next(), PIECES.len())];
                        input.splice(at..at, piece.iter().copied());
                    }
                }
            }
            let whole = read_response(input.as_slice(), &limits);
            seen[match &whole {
                Ok(_) => 0,
                Err(Failure::MalformedResponse(_)) => 1,
                Err(_) => 2,
            }] += 1;
            let trickled = read_response(
                Trickle {
                    bytes: &input,
                    step: 1,
                },
                &limits,
            );
            assert_eq!(
                whole,
                trickled,
                "input {:?}",
                String::from_utf8_lossy(&input)
            );
        }
        // Each outcome was reached, so the inputs exercised more than the first byte.
        assert!(seen.iter().all(|&n| n > 0), "outcomes {seen:?}");
    }

    #[test]
    fn local_addresses_are_the_classes_of_the_destination_rule() {
        let local = [
            "127.0.0.1",
            "127.255.255.254",
            "10.0.0.1",
            "10.255.255.255",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.0.1",
            "169.254.169.254",
            "100.64.0.1",
            "100.127.255.255",
            "0.0.0.0",
            "::1",
            "::",
            "fe80::1",
            "febf::1",
            "fc00::1",
            "fdff::1",
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
            "::ffff:169.254.169.254",
            "::ffff:100.64.0.1",
            "::ffff:0.0.0.0",
            "::127.0.0.1",
            "64:ff9b::7f00:1",
            "2002:a9fe:a9fe::1",
        ];
        let public = [
            "203.0.113.5",
            "198.51.100.7",
            "8.8.8.8",
            "172.15.255.255",
            "172.32.0.0",
            "192.167.255.255",
            "169.253.0.1",
            "100.63.255.255",
            "100.128.0.0",
            "11.0.0.1",
            "0.0.0.1",
            "2001:db8::1",
            "fec0::1",
            "fe7f::1",
            "fb00::1",
            "::ffff:203.0.113.5",
            "::203.0.113.5",
            "64:ff9b::cb00:7105",
            "2002:cb00:7105::1",
        ];
        for text in local {
            let ip: IpAddr = text.parse().unwrap();
            assert!(is_local(ip), "{text} is local");
        }
        for text in public {
            let ip: IpAddr = text.parse().unwrap();
            assert!(!is_local(ip), "{text} is public");
        }
    }
}
