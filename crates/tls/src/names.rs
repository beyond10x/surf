//! Host names: whether a server's certificate is for the host asked for (RFC 9525;
//! docs/design/tls.md, items 6 and 7; `story:tls-x509-path`).
//!
//! The reference identifier is the host of the address, as [`host`] reads it: an IP address
//! (four decimal octets, or an IPv6 address, bracketed or not) or else a DNS name. The
//! certificate's identifiers are the entries of its subjectAltName, nothing else: the Common
//! Name is never read (RFC 9525 section 6.4.4 allows a client to leave it; surf does).
//!
//! - A DNS name matches a dNSName entry when both are names in the preferred name syntax
//!   ([`is_dns_name`]) and are equal without regard to ASCII case (section 6.3).
//! - A dNSName entry may hold `*` only as its whole left-most label, with at least two labels
//!   after it; it then matches a host of exactly one more label (section 6.3: the wildcard
//!   matches one label). `f*.example.test`, `*.*.test`, `*` and `*.test` match nothing. The
//!   rule of two labels is surf's own: without a public suffix list it keeps a wildcard off a
//!   top-level domain.
//! - An IP host matches an iPAddress entry of the same length, octet for octet (4 or 16), and
//!   never a dNSName entry (section 6.2.1, 6.4.1).
//!
//! The work is one comparison per entry, so a certificate with n entries costs n
//! ([`matches_counting`]).

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::x509::{Certificate, GeneralName};

/// A host as the reference identifier of RFC 9525.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Host<'a> {
    /// A DNS name, as given.
    Dns(&'a str),
    /// An IP address: its 4 or 16 octets.
    Ip(Vec<u8>),
}

impl fmt::Display for Host<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Host::Dns(name) => f.write_str(name),
            Host::Ip(octets) => f.write_str(&ip_text(octets)),
        }
    }
}

/// The host `text` as a reference identifier: an IP address when it is one in the forms
/// [`std::net`] reads (an IPv4 address is four decimal octets without leading zeros; an IPv6
/// address may stand in brackets), else a DNS name.
pub fn host(text: &str) -> Host<'_> {
    let inner = text
        .strip_prefix('[')
        .and_then(|t| t.strip_suffix(']'))
        .unwrap_or(text);
    if inner.len() != text.len() {
        return match inner.parse::<Ipv6Addr>() {
            Ok(v6) => Host::Ip(v6.octets().to_vec()),
            Err(_) => Host::Dns(text),
        };
    }
    match text.parse::<IpAddr>() {
        Ok(IpAddr::V4(v4)) => Host::Ip(v4.octets().to_vec()),
        Ok(IpAddr::V6(v6)) => Host::Ip(v6.octets().to_vec()),
        Err(_) => Host::Dns(text),
    }
}

/// An IP address of 4 or 16 octets as text; other lengths as hex octets joined by `:`.
pub fn ip_text(octets: &[u8]) -> String {
    match octets.len() {
        4 => Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]).to_string(),
        16 => {
            let mut a = [0u8; 16];
            a.copy_from_slice(octets);
            Ipv6Addr::from(a).to_string()
        }
        _ => octets
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(":"),
    }
}

/// Whether `name` is a DNS name in the preferred name syntax of RFC 5280 section 4.2.1.6 (RFC
/// 1034 section 3.5 as RFC 1123 section 2.1 relaxes it): labels of 1 to 63 letters, digits and
/// hyphens, neither starting nor ending with a hyphen, joined by single dots, at most 253 bytes
/// in all, no dot at either end, and a last label that is no number in the sense of the URL
/// Standard's IPv4 parser ([`is_url_number`]), so that no IPv4 address in text, in any form a
/// URL host parser reads (`127.0.0.1`, `0x7f000001`, `127.0.0.0x1`), is a DNS name.
pub fn is_dns_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 253 {
        return false;
    }
    let mut last = "";
    for label in name.split('.') {
        let bytes = label.as_bytes();
        if bytes.is_empty()
            || bytes.len() > 63
            || bytes[0] == b'-'
            || bytes[bytes.len() - 1] == b'-'
            || !bytes
                .iter()
                .all(|&b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return false;
        }
        last = label;
    }
    !is_url_number(last)
}

/// Whether `label` is a number as the URL Standard's IPv4 parser reads one ("ends in a
/// number"): ASCII digits only, or `0x` or `0X` followed by hex digits or by nothing.
pub fn is_url_number(label: &str) -> bool {
    if let Some(hex) = label
        .strip_prefix("0x")
        .or_else(|| label.strip_prefix("0X"))
    {
        return hex.bytes().all(|b| b.is_ascii_hexdigit());
    }
    !label.is_empty() && label.bytes().all(|b| b.is_ascii_digit())
}

/// The suffix of a wildcard dNSName `*.<suffix>` whose suffix is a DNS name of at least two
/// labels; `None` for any other name.
pub fn wildcard_suffix(pattern: &str) -> Option<&str> {
    let suffix = pattern.strip_prefix("*.")?;
    (is_dns_name(suffix) && suffix.contains('.')).then_some(suffix)
}

/// Whether the dNSName entry `pattern` matches the DNS name `host` (both checked here).
pub fn dns_name_matches(pattern: &str, host: &str) -> bool {
    if !is_dns_name(host) {
        return false;
    }
    if let Some(suffix) = wildcard_suffix(pattern) {
        return match host.split_once('.') {
            Some((_, rest)) => rest.eq_ignore_ascii_case(suffix),
            None => false,
        };
    }
    is_dns_name(pattern) && pattern.eq_ignore_ascii_case(host)
}

/// Whether an entry of `names` (a subjectAltName) matches `host`, and the number of entries
/// compared: up to and including the one that matches, or all of them.
pub fn matches_counting(names: &[GeneralName<'_>], host_text: &str) -> (bool, usize) {
    let reference = host(host_text);
    let mut compared = 0;
    for name in names {
        compared += 1;
        let hit = match (name, &reference) {
            (GeneralName::Dns(pattern), Host::Dns(dns)) => dns_name_matches(pattern, dns),
            (GeneralName::Ip(octets), Host::Ip(ip)) => *octets == ip.as_slice(),
            _ => false,
        };
        if hit {
            return (true, compared);
        }
    }
    (false, compared)
}

/// Whether an entry of `names` (a subjectAltName) matches `host`.
pub fn matches(names: &[GeneralName<'_>], host_text: &str) -> bool {
    matches_counting(names, host_text).0
}

/// Whether an entry of the subjectAltName of `cert` matches `host`; a certificate without one
/// matches nothing.
pub fn certificate_matches(cert: &Certificate<'_>, host_text: &str) -> bool {
    cert.extensions
        .subject_alt_name
        .as_deref()
        .is_some_and(|names| matches(names, host_text))
}
