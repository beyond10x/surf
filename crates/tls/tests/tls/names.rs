//! Host names (RFC 9525, `surf_tls::names`; `story:tls-x509-path` acceptance 3 and 4): a
//! dNSName matches without regard to ASCII case, `*` only as the whole left-most label and for
//! one label, an IP host matches an iPAddress entry octet for octet and never a dNSName, and
//! the Common Name is never read.

#![forbid(unsafe_code)]

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use surf_tls::names::{self, Host};
use surf_tls::pem;
use surf_tls::x509::{Certificate, GeneralName};

fn dns(name: &str) -> GeneralName<'_> {
    GeneralName::Dns(name)
}

fn matches(sans: &[GeneralName<'_>], host: &str) -> bool {
    names::matches(sans, host)
}

#[test]
fn a_wildcard_matches_one_left_most_label() {
    let sans = [dns("*.example.test")];
    assert!(matches(&sans, "a.example.test"));
    assert!(!matches(&sans, "example.test"));
    assert!(!matches(&sans, "a.b.example.test"));
    assert!(!matches(&sans, ".example.test"));
}

#[test]
fn a_star_inside_a_label_matches_nothing() {
    let sans = [dns("f*.example.test")];
    for host in [
        "f.example.test",
        "fo.example.test",
        "foo.example.test",
        "f*.example.test",
    ] {
        assert!(!matches(&sans, host), "{host}");
    }
    for pattern in [
        "*o.example.test",
        "a.*.example.test",
        "*",
        "*.test",
        "**.example.test",
    ] {
        assert!(!matches(&[dns(pattern)], "fo.example.test"), "{pattern}");
        assert!(!matches(&[dns(pattern)], "a.b.example.test"), "{pattern}");
        assert!(!matches(&[dns(pattern)], "x.test"), "{pattern}");
    }
}

#[test]
fn dns_names_match_without_regard_to_ascii_case() {
    assert!(matches(&[dns("EXAMPLE.test")], "example.TEST"));
    assert!(matches(&[dns("*.Example.Test")], "A.EXAMPLE.test"));
    assert!(!matches(&[dns("example.test")], "example.tests"));
    assert!(!matches(&[dns("example.test")], "xexample.test"));
}

#[test]
fn an_ip_host_matches_its_octets_and_never_a_dns_name() {
    let v4 = [GeneralName::Ip(&[127, 0, 0, 1])];
    assert!(matches(&v4, "127.0.0.1"));
    assert!(!matches(&v4, "127.0.0.2"));
    assert!(!matches(&[dns("127.0.0.1")], "127.0.0.1"));
    let mut loopback = [0u8; 16];
    loopback[15] = 1;
    let v6 = [GeneralName::Ip(&loopback)];
    assert!(matches(&v6, "::1"));
    assert!(matches(&v6, "[::1]"));
    assert!(matches(&v6, "0:0:0:0:0:0:0:1"));
    assert!(!matches(&v6, "::2"));
    // The 4 octets of 127.0.0.1 are not the 16 of ::ffff:127.0.0.1, nor the other way round.
    let mut mapped = [0u8; 16];
    mapped[10] = 0xFF;
    mapped[11] = 0xFF;
    mapped[12..].copy_from_slice(&[127, 0, 0, 1]);
    assert!(!matches(&[GeneralName::Ip(&mapped)], "127.0.0.1"));
    assert!(!matches(&v4, "::ffff:127.0.0.1"));
}

#[test]
fn a_dns_host_never_matches_an_ip_entry() {
    assert!(!matches(&[GeneralName::Ip(&[127, 0, 0, 1])], "localhost"));
}

#[test]
fn hosts_and_names_outside_the_preferred_syntax_match_nothing() {
    for (pattern, host) in [
        ("foo_bar.example.test", "foo_bar.example.test"),
        ("-a.example.test", "-a.example.test"),
        ("a-.example.test", "a-.example.test"),
        ("a..example.test", "a..example.test"),
        ("example.test.", "example.test."),
        ("", ""),
        ("1.2.3", "1.2.3"),
    ] {
        assert!(!matches(&[dns(pattern)], host), "{pattern} {host}");
    }
    let long_label = "a".repeat(64);
    let name = format!("{long_label}.test");
    assert!(!matches(&[dns(&name)], &name));
    let ok_label = "a".repeat(63);
    let name = format!("{ok_label}.test");
    assert!(matches(&[dns(&name)], &name));
}

#[test]
fn other_kinds_of_names_match_nothing() {
    let other = [GeneralName::Other {
        tag: 0x81,
        content: b"localhost",
    }];
    assert!(!matches(&other, "localhost"));
}

#[test]
fn the_host_is_read_as_an_ip_address_or_a_dns_name() {
    assert_eq!(names::host("127.0.0.1"), Host::Ip(vec![127, 0, 0, 1]));
    assert_eq!(names::host("[::1]").to_string(), "::1");
    assert_eq!(names::host("Example.Test"), Host::Dns("Example.Test"));
    // Forms of an IPv4 address that are not four decimal octets are no address.
    assert_eq!(names::host("127.1"), Host::Dns("127.1"));
    assert_eq!(names::host("0x7f.0.0.1"), Host::Dns("0x7f.0.0.1"));
    assert_eq!(names::host("127.000.0.1"), Host::Dns("127.000.0.1"));
}

/// Acceptance 4: `1000-sans` matches on its last entry; the number of entries compared is the
/// cost, and the time is printed for the report.
#[test]
fn a_thousand_names_match_on_the_last_in_linear_work() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tls-test-certs/certs/1000-sans");
    let chain = fs::read(dir.join("chain.pem")).expect("read chain.pem");
    let blocks = pem::parse(&chain).expect("PEM");
    let leaf = Certificate::from_der(&blocks[0].data).expect("the leaf reads");
    let sans = leaf.extensions.subject_alt_name.expect("a subjectAltName");
    assert_eq!(sans.len(), 1000);
    assert_eq!(sans[999], GeneralName::Dns("localhost"));
    let started = Instant::now();
    let (found, compared) = names::matches_counting(&sans, "localhost");
    let took = started.elapsed();
    println!("1000-sans: matched on entry {compared} in {took:?}");
    assert!(found);
    assert_eq!(
        compared, 1000,
        "every entry compared once, the last matches"
    );
    let (found, compared) = names::matches_counting(&sans, "nowhere.example.test");
    assert!(!found);
    assert_eq!(compared, 1000);
    let (found, compared) = names::matches_counting(&sans[..500], "nowhere.example.test");
    assert!(!found);
    assert_eq!(compared, 500, "half the entries, half the work");
}

/// The Common Name is never read: `common-name-only` has `localhost` there and no
/// subjectAltName, and nothing matches.
#[test]
fn the_common_name_is_never_read() {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tls-test-certs/certs/common-name-only");
    let chain = fs::read(dir.join("chain.pem")).expect("read chain.pem");
    let blocks = pem::parse(&chain).expect("PEM");
    let leaf = Certificate::from_der(&blocks[0].data).expect("the leaf reads");
    assert!(leaf.extensions.subject_alt_name.is_none());
    assert!(
        leaf.subject
            .der()
            .windows(b"localhost".len())
            .any(|w| w == b"localhost"),
        "the subject holds localhost"
    );
    assert!(!names::certificate_matches(&leaf, "localhost"));
}
