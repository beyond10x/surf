//! The adversary's cases of `story:agent-cookie-jar` for claims of its rules that no case U1 to
//! U14 held: a `Path` attribute without a leading `/` (R5), a `Max-Age` too large for a number
//! (R3), a full domain of `Secure` cookies (R13), the `Cookie` field cut from the end of its
//! order (R14) and the bounds of the cookie-date algorithm (R3, draft section 5.1.1). The jar is
//! called directly, the time given; every cookie is a constant of the test.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use surf_cookie::{Jar, Method, Site, parse_date};
use surf_url::Url;

/// 2026-11-22T00:00:00Z.
const T: u64 = 1_795_305_600;

/// T plus 400 days.
const T_400_DAYS: u64 = 1_829_865_600;

fn url(address: &str) -> Url {
    Url::parse(address, None).expect("a test address parses")
}

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

/// The `Cookie` field for `address` at `now`, same-site, `GET`.
fn field(jar: &Jar, address: &str, now: u64) -> Option<String> {
    jar.cookie_field(&url(address), Site::Same, Method::Get, at(now))
}

/// R5: a `Path` attribute that does not start with `/` gives the default-path of the request
/// address, `/d/e` for `/d/e/set`.
#[test]
fn a_path_without_a_leading_slash_gives_the_default_path() {
    let jar = Jar::new();
    jar.receive(&url("http://h.test/d/e/set"), "r=1; Path=rel", at(T));
    assert_eq!(
        field(&jar, "http://h.test/d/e/x", T).as_deref(),
        Some("r=1")
    );
    assert_eq!(field(&jar, "http://h.test/d/x", T), None);
}

/// R3: a `Max-Age` of more digits than a 64-bit number holds is a lifetime over 400 days, cut
/// to 400 days.
#[test]
fn a_max_age_too_large_for_a_number_lasts_400_days() {
    let jar = Jar::new();
    jar.receive(
        &url("http://h.test/"),
        "big=1; Max-Age=99999999999999999999",
        at(T),
    );
    assert_eq!(
        field(&jar, "http://h.test/", T_400_DAYS - 1).as_deref(),
        Some("big=1")
    );
    assert_eq!(field(&jar, "http://h.test/", T_400_DAYS + 1), None);
}

/// R13: a domain over its bound with no cookie without `Secure` evicts "any cookie of such a
/// domain", the least recently used.
#[test]
fn a_full_domain_of_secure_cookies_evicts_the_least_recently_used() {
    let jar = Jar::new();
    for n in 1..=51 {
        jar.receive(&url("https://h.test/"), &format!("s{n}=1; Secure"), at(T));
    }
    assert_eq!(jar.count(at(T)), 50);
    let sent = field(&jar, "https://h.test/", T).expect("h keeps cookies");
    assert!(
        sent.starts_with("s2=1; s3=1"),
        "{}",
        &sent[..sent.len().min(40)]
    );
    assert!(
        sent.ends_with("; s51=1"),
        "{}",
        &sent[sent.len().saturating_sub(40)..]
    );
}

/// R14: cookies are dropped from the end of R12's order until the field fits, so a small
/// cookie after one that does not fit is dropped too.
#[test]
fn the_cookie_field_is_cut_from_the_end_of_its_order() {
    let jar = Jar::new();
    let [a, b, c] = [("a", "x"), ("b", "y"), ("c", "z")]
        .map(|(name, fill)| format!("{name}={}", fill.repeat(4000)));
    for cookie in [&a, &b, &c] {
        jar.receive(&url("http://h.test/"), &format!("{cookie}; Path=/"), at(T));
    }
    jar.receive(&url("http://h.test/"), "d=1; Path=/", at(T));
    assert_eq!(field(&jar, "http://h.test/", T), Some(format!("{a}; {b}")));
}

/// R3, draft section 5.1.1: a two-digit year of 70 to 99 is of the 1900s, of 0 to 69 of the
/// 2000s; a day of month outside 1 to 31 fails; the last second of a year parses.
#[test]
fn dates_at_the_bounds_of_the_cookie_date_algorithm() {
    assert_eq!(parse_date("Thu, 01 Jan 70 00:00:00 GMT"), Some(0));
    assert_eq!(
        parse_date("Tue, 01 Jan 69 00:00:00 GMT"),
        Some(3_124_224_000)
    );
    assert_eq!(parse_date("31 Dec 2030 23:59:59"), Some(1_924_991_999));
    assert_eq!(parse_date("00 Jan 2030 00:00:00"), None);
    assert_eq!(parse_date("32 Jan 2030 00:00:00"), None);
}
