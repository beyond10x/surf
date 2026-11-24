//! Security review of `story:agent-cookie-jar`, the jar called directly: what the unit cases U1
//! to U14 leave open. A cookie of a parent host never reaches a child host (R4, host-only); a
//! `Set-Cookie` string with CR, LF or NUL is never stored, so no `Cookie` field the fetch writes
//! can carry a line break (R1, header injection); a hostile date never panics (R3). Every
//! cookie here is a constant of the test.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use surf_cookie::{Jar, Method, Site, parse_date};
use surf_url::Url;

/// 2026-11-22T00:00:00Z, the T of the story.
const T: u64 = 1_795_305_600;

fn url(address: &str) -> Url {
    Url::parse(address, None).expect("a test address parses")
}

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

fn field(jar: &Jar, address: &str) -> Option<String> {
    jar.cookie_field(&url(address), Site::Same, Method::Get, at(T))
}

#[test]
fn a_parent_hosts_cookie_never_reaches_a_child_host() {
    let jar = Jar::new();
    for set_cookie in [
        "plain=1; Path=/",
        "dom=1; Domain=example.test; Path=/",
        "dot=1; Domain=.example.test; Path=/",
    ] {
        jar.receive(&url("http://example.test/"), set_cookie, at(T));
    }
    jar.receive(&url("http://127.0.0.1/"), "ip=1; Path=/", at(T));
    assert_eq!(
        field(&jar, "http://example.test/").as_deref(),
        Some("plain=1; dom=1; dot=1"),
        "the setting host gets its cookies back"
    );
    for other in [
        "http://a.example.test/",
        "http://b.a.example.test/",
        "http://xexample.test/",
        "http://example.test.evil.test/",
        "http://0.0.1/",
    ] {
        assert_eq!(field(&jar, other), None, "a cookie reached {other}");
    }
    assert_eq!(field(&jar, "http://127.0.0.1/").as_deref(), Some("ip=1"));
}

#[test]
fn a_set_cookie_with_cr_lf_or_nul_is_never_stored() {
    let jar = Jar::new();
    for set_cookie in [
        "lf=1\nInjected: yes; Path=/",
        "cr=1\rInjected: yes; Path=/",
        "crlf=1\r\nInjected: yes; Path=/",
        "nul=1\0x; Path=/",
        "attr=1; Path=/\r\nInjected: yes",
        "na\nme=1; Path=/",
    ] {
        jar.receive(&url("http://h.test/"), set_cookie, at(T));
    }
    assert_eq!(field(&jar, "http://h.test/"), None);
    assert_eq!(jar.count(at(T)), 0);
}

#[test]
fn a_hostile_date_never_panics() {
    let long_digits = "9".repeat(40);
    let hostile = [
        format!("06 Nov {long_digits} 08:49:37"),
        format!("{long_digits} Nov 1994 08:49:37"),
        format!("06 Nov 1994 {long_digits}:49:37"),
        "06 Nov 9999 23:59:59".to_string(),
        "31 Dec 1601 00:00:00".to_string(),
        "29 Feb 2100 00:00:00".to_string(),
        ":::: ;;;; ----".to_string(),
        "Nov Nov Nov 1 1 1 1:1:1 1:1:1".to_string(),
    ];
    for text in &hostile {
        let _ = parse_date(text);
    }
    assert_eq!(parse_date(&hostile[0]), None);
    assert_eq!(
        parse_date("29 Feb 2100 00:00:00"),
        None,
        "2100 is no leap year"
    );
    let jar = Jar::new();
    for text in &hostile {
        jar.receive(
            &url("http://h.test/"),
            &format!("d=1; Path=/; Expires={text}"),
            at(T),
        );
    }
    assert_eq!(jar.count(at(T)), 1);
}
