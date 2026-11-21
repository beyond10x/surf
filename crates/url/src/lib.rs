//! The WHATWG URL parser surf resolves addresses with, written with the Rust standard library
//! only.
//!
//! [`Url::parse`] runs the URL Standard's basic URL parser on an input, against an optional base
//! URL, and returns a URL record or a [`ParseError`]. [`Url::href`] serializes it,
//! [`Url::origin`] gives its origin and [`Origin::same_origin`] compares two.
//! [`form_urlencoded::serialize`] is the `application/x-www-form-urlencoded` serializer over name
//! and value pairs.
//!
//! The standard is the WHATWG URL Standard (<https://url.spec.whatwg.org/>), read at commit
//! `4f461b81a748443b235840599cea6290df40329a` of `https://github.com/whatwg/url`. The tests are
//! web-platform-tests' `url/resources/urltestdata.json`, vendored in `tests/data/` with its
//! source in `tests/data/SOURCE`; `tests/urltestdata.rs` runs every case of it.
//!
//! # Sections followed, and none claimed complete
//!
//! A section is marked complete only when every requirement in it is written and every case the
//! test data has for it passes. The cases skipped for domain to ASCII (`tests/data/SKIPPED`)
//! also check parsing and serializing, so no section is marked complete.
//!
//! - "Percent-encoded bytes": followed for UTF-8. Incomplete: no other output encoding.
//! - "Hosts (domains and IP addresses)": "Host parsing", "Host serializing" and "Host
//!   equivalence" ([`Host`]'s `==`) followed. Incomplete: no domain to ASCII (section "IDNA");
//!   no public suffix or registrable domain.
//! - "URL parsing": the basic URL parser, with UTF-8 as its only encoding, and without the `url`
//!   and `state override` arguments (they serve the setters of the `URL` interface). Incomplete
//!   for those and for the blob URL entry, which nothing here resolves.
//! - "URL serializing": the URL serializer and the URL path serializer, all of it.
//! - "Origin": followed. A `file:` URL's origin is a new opaque origin, the standard's "when in
//!   doubt". Incomplete: no blob URL entry.
//! - "`application/x-www-form-urlencoded` serializing": followed for UTF-8. Incomplete: no
//!   other encoding.
//!
//! # What this crate does not do
//!
//! - Domain to ASCII (IDNA, UTS #46). A special URL whose host, percent-decoded, holds a code
//!   point outside ASCII is [`ParseError::NonAsciiHost`]. An ASCII domain needs no IDNA: the
//!   standard's domain parser returns it lower-cased (section "IDNA", the domain parser, step 5).
//!   An opaque host (a URL that is not special) is percent-encoded and needs none either.
//! - Validation errors that do not end parsing are not reported. A failure names the validation
//!   error that ended it ([`ValidationError`]).
//! - The `URL` and `URLSearchParams` interfaces and their setters; the
//!   `application/x-www-form-urlencoded` parser; URL equivalence; URL rendering.
//!
//! # Limits
//!
//! An input longer than [`MAX_INPUT_LENGTH`] bytes is [`ParseError::TooLong`]; the standard sets
//! no limit, this crate does. The result's serialization is then at most about three times the
//! input and the base together, since percent-encoding turns one byte into three. No input
//! makes this crate panic: every index is checked, and the numbers of IPv4 addresses and ports
//! are read with saturating arithmetic.

#![forbid(unsafe_code)]

mod host;
mod parser;
mod percent;

pub mod form_urlencoded;

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

pub use host::Host;

/// The longest input [`Url::parse`] reads, in bytes of UTF-8: 2 MiB.
pub const MAX_INPUT_LENGTH: usize = 2 * 1024 * 1024;

/// A URL record.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Url {
    scheme: String,
    username: String,
    password: String,
    host: Option<Host>,
    port: Option<u16>,
    path: Path,
    query: Option<String>,
    fragment: Option<String>,
}

/// A URL's path: a list of segments, or one opaque segment.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Path {
    /// The list of URL path segments.
    Segments(Vec<String>),
    /// An opaque path, as in `mailto:someone@example.org`.
    Opaque(String),
}

/// Why an input is not a URL.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParseError {
    /// The URL Standard's parser returns failure; the validation error that ends it.
    Invalid(ValidationError),
    /// A special URL's host holds a code point outside ASCII after percent-decoding. It needs
    /// domain to ASCII (IDNA), which this crate does not do; the standard may accept it.
    NonAsciiHost,
    /// The input is longer than [`MAX_INPUT_LENGTH`] bytes.
    TooLong,
}

/// The validation errors that end the parser with failure, by their name in the URL Standard.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ValidationError {
    /// `domain-to-ASCII`: the domain is empty.
    DomainToAscii,
    /// `domain-invalid-code-point`
    DomainInvalidCodePoint,
    /// `host-invalid-code-point`
    HostInvalidCodePoint,
    /// `IPv4-too-many-parts`
    Ipv4TooManyParts,
    /// `IPv4-non-numeric-part`
    Ipv4NonNumericPart,
    /// `IPv4-out-of-range-part`
    Ipv4OutOfRangePart,
    /// `IPv6-unclosed`
    Ipv6Unclosed,
    /// `IPv6-invalid-compression`
    Ipv6InvalidCompression,
    /// `IPv6-too-many-pieces`
    Ipv6TooManyPieces,
    /// `IPv6-multiple-compression`
    Ipv6MultipleCompression,
    /// `IPv6-invalid-code-point`
    Ipv6InvalidCodePoint,
    /// `IPv6-too-few-pieces`
    Ipv6TooFewPieces,
    /// `IPv4-in-IPv6-too-many-pieces`
    Ipv4InIpv6TooManyPieces,
    /// `IPv4-in-IPv6-invalid-code-point`
    Ipv4InIpv6InvalidCodePoint,
    /// `IPv4-in-IPv6-out-of-range-part`
    Ipv4InIpv6OutOfRangePart,
    /// `IPv4-in-IPv6-too-few-parts`
    Ipv4InIpv6TooFewParts,
    /// `missing-scheme-non-relative-URL`
    MissingSchemeNonRelativeUrl,
    /// `host-missing`
    HostMissing,
    /// `port-out-of-range`
    PortOutOfRange,
    /// `port-invalid`
    PortInvalid,
}

impl ValidationError {
    /// The validation error's name in the URL Standard.
    pub fn name(self) -> &'static str {
        match self {
            ValidationError::DomainToAscii => "domain-to-ASCII",
            ValidationError::DomainInvalidCodePoint => "domain-invalid-code-point",
            ValidationError::HostInvalidCodePoint => "host-invalid-code-point",
            ValidationError::Ipv4TooManyParts => "IPv4-too-many-parts",
            ValidationError::Ipv4NonNumericPart => "IPv4-non-numeric-part",
            ValidationError::Ipv4OutOfRangePart => "IPv4-out-of-range-part",
            ValidationError::Ipv6Unclosed => "IPv6-unclosed",
            ValidationError::Ipv6InvalidCompression => "IPv6-invalid-compression",
            ValidationError::Ipv6TooManyPieces => "IPv6-too-many-pieces",
            ValidationError::Ipv6MultipleCompression => "IPv6-multiple-compression",
            ValidationError::Ipv6InvalidCodePoint => "IPv6-invalid-code-point",
            ValidationError::Ipv6TooFewPieces => "IPv6-too-few-pieces",
            ValidationError::Ipv4InIpv6TooManyPieces => "IPv4-in-IPv6-too-many-pieces",
            ValidationError::Ipv4InIpv6InvalidCodePoint => "IPv4-in-IPv6-invalid-code-point",
            ValidationError::Ipv4InIpv6OutOfRangePart => "IPv4-in-IPv6-out-of-range-part",
            ValidationError::Ipv4InIpv6TooFewParts => "IPv4-in-IPv6-too-few-parts",
            ValidationError::MissingSchemeNonRelativeUrl => "missing-scheme-non-relative-URL",
            ValidationError::HostMissing => "host-missing",
            ValidationError::PortOutOfRange => "port-out-of-range",
            ValidationError::PortInvalid => "port-invalid",
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Invalid(error) => write!(f, "not a URL: {}", error.name()),
            ParseError::NonAsciiHost => {
                f.write_str("a host outside ASCII needs domain to ASCII (IDNA), not supported")
            }
            ParseError::TooLong => write!(f, "longer than {MAX_INPUT_LENGTH} bytes"),
        }
    }
}

impl std::error::Error for ParseError {}

/// The special schemes and their default ports.
fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "ftp" => Some(21),
        "http" | "ws" => Some(80),
        "https" | "wss" => Some(443),
        _ => None,
    }
}

fn is_special_scheme(scheme: &str) -> bool {
    matches!(scheme, "ftp" | "file" | "http" | "https" | "ws" | "wss")
}

impl Url {
    /// The basic URL parser: `input` against `base`, if any.
    ///
    /// The `URL` constructor's "API URL parser" parses its base string first and fails when
    /// that fails; a caller holding a base string does the same with `Url::parse(base, None)`.
    pub fn parse(input: &str, base: Option<&Url>) -> Result<Url, ParseError> {
        parser::parse(input, base)
    }

    /// The scheme, ASCII lower case, without the `:`.
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    /// The username, percent-encoded; empty when there is none.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// The password, percent-encoded; empty when there is none.
    pub fn password(&self) -> &str {
        &self.password
    }

    /// The host, or `None`.
    pub fn host(&self) -> Option<&Host> {
        self.host.as_ref()
    }

    /// The port, or `None` for none and for the scheme's default port.
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// The path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The query without its `?`, or `None`.
    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    /// The fragment without its `#`, or `None`.
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }

    /// Whether the scheme is a special scheme (`ftp`, `file`, `http`, `https`, `ws`, `wss`).
    pub fn is_special(&self) -> bool {
        is_special_scheme(&self.scheme)
    }

    /// The URL serializer.
    pub fn href(&self) -> String {
        self.serialize(false)
    }

    /// The URL serializer, with its `exclude fragment` argument.
    pub fn serialize(&self, exclude_fragment: bool) -> String {
        let mut output = String::with_capacity(self.scheme.len() + 16);
        output.push_str(&self.scheme);
        output.push(':');
        if let Some(host) = &self.host {
            output.push_str("//");
            if !self.username.is_empty() || !self.password.is_empty() {
                output.push_str(&self.username);
                if !self.password.is_empty() {
                    output.push(':');
                    output.push_str(&self.password);
                }
                output.push('@');
            }
            output.push_str(&host.serialize());
            if let Some(port) = self.port {
                output.push(':');
                output.push_str(&port.to_string());
            }
        }
        if self.host.is_none() {
            if let Path::Segments(segments) = &self.path {
                if segments.len() > 1 && segments[0].is_empty() {
                    output.push_str("/.");
                }
            }
        }
        output.push_str(&self.serialize_path());
        if let Some(query) = &self.query {
            output.push('?');
            output.push_str(query);
        }
        if !exclude_fragment {
            if let Some(fragment) = &self.fragment {
                output.push('#');
                output.push_str(fragment);
            }
        }
        output
    }

    /// The URL path serializer.
    pub fn serialize_path(&self) -> String {
        match &self.path {
            Path::Opaque(path) => path.clone(),
            Path::Segments(segments) => {
                let mut output = String::new();
                for segment in segments {
                    output.push('/');
                    output.push_str(segment);
                }
                output
            }
        }
    }

    /// The URL's origin.
    pub fn origin(&self) -> Origin {
        match self.scheme.as_str() {
            "blob" => match Url::parse(&self.serialize_path(), None) {
                Ok(path_url) if matches!(path_url.scheme(), "http" | "https" | "file") => {
                    path_url.origin()
                }
                _ => Origin::new_opaque(),
            },
            "ftp" | "http" | "https" | "ws" | "wss" => match &self.host {
                Some(host) => Origin::Tuple {
                    scheme: self.scheme.clone(),
                    host: host.clone(),
                    port: self.port,
                },
                None => Origin::new_opaque(),
            },
            _ => Origin::new_opaque(),
        }
    }
}

/// An origin (HTML Standard, "Origins"): a tuple of scheme, host and port, or an opaque origin.
/// The tuple's domain is always null here.
#[derive(Clone, Debug)]
pub enum Origin {
    /// An opaque origin. Each new one is distinct; a clone is the same origin.
    Opaque(OpaqueOrigin),
    /// A tuple origin.
    Tuple {
        /// The scheme.
        scheme: String,
        /// The host.
        host: Host,
        /// The port, `None` for the scheme's default.
        port: Option<u16>,
    },
}

/// The identity of an opaque origin.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OpaqueOrigin(u64);

static NEXT_OPAQUE_ORIGIN: AtomicU64 = AtomicU64::new(0);

impl Origin {
    /// A new opaque origin, same origin only with itself and its clones.
    pub fn new_opaque() -> Origin {
        Origin::Opaque(OpaqueOrigin(
            NEXT_OPAQUE_ORIGIN.fetch_add(1, Ordering::Relaxed),
        ))
    }

    /// Same origin (HTML Standard): the same opaque origin, or two tuple origins with equal
    /// scheme, host and port.
    pub fn same_origin(&self, other: &Origin) -> bool {
        match (self, other) {
            (Origin::Opaque(a), Origin::Opaque(b)) => a == b,
            (
                Origin::Tuple { scheme, host, port },
                Origin::Tuple {
                    scheme: other_scheme,
                    host: other_host,
                    port: other_port,
                },
            ) => scheme == other_scheme && host == other_host && port == other_port,
            _ => false,
        }
    }

    /// The serialization of an origin (HTML Standard): `null` for an opaque origin, otherwise
    /// the scheme, `://`, the host serialized and, when there is one, `:` and the port.
    pub fn serialize(&self) -> String {
        match self {
            Origin::Opaque(_) => "null".to_string(),
            Origin::Tuple { scheme, host, port } => match port {
                Some(port) => format!("{scheme}://{}:{port}", host.serialize()),
                None => format!("{scheme}://{}", host.serialize()),
            },
        }
    }
}
