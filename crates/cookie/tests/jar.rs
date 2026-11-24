//! The unit cases U1 to U14 of `story:agent-cookie-jar`: the jar called directly, the time given.
//! T is 1795305600 (2026-11-22T00:00:00Z).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use surf_cookie::{Jar, Method, Site, parse_date, site};
use surf_url::Url;

const T: u64 = 1_795_305_600;

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

/// Each `Set-Cookie` field of `fields` received from `address` at `now`.
fn receive(jar: &Jar, address: &str, fields: &[&str], now: u64) {
    for set_cookie in fields {
        jar.receive(&url(address), set_cookie, at(now));
    }
}

#[test]
fn u01_dates_parse_by_the_cookie_date_algorithm() {
    for text in [
        "Sun, 06 Nov 1994 08:49:37 GMT",
        "Sunday, 06-Nov-94 08:49:37 GMT",
        "Sun Nov  6 08:49:37 1994",
    ] {
        assert_eq!(parse_date(text), Some(784_111_777), "{text}");
    }
    for text in [
        "06 Nov 1600 08:49:37",
        "31 Feb 2030 08:49:37",
        "06 Nov 1994 24:00:00",
    ] {
        assert_eq!(parse_date(text), None, "{text}");
    }
}

#[test]
fn u02_max_age_wins_and_a_bad_one_is_ignored() {
    let jar = Jar::new();
    receive(
        &jar,
        "http://h.test/",
        &["m=1; Max-Age=60; Expires=Thu, 01 Jan 1970 00:00:01 GMT"],
        T,
    );
    assert_eq!(
        field(&jar, "http://h.test/", T + 59).as_deref(),
        Some("m=1")
    );
    assert_eq!(field(&jar, "http://h.test/", T + 61), None);

    let jar = Jar::new();
    receive(&jar, "http://h.test/", &["n=1; Max-Age=-1"], T);
    assert_eq!(field(&jar, "http://h.test/", T), None);

    let jar = Jar::new();
    receive(&jar, "http://h.test/", &["o=1; Max-Age=1x"], T);
    assert_eq!(
        field(&jar, "http://h.test/", T + 1_000_000_000).as_deref(),
        Some("o=1")
    );
}

#[test]
fn u03_a_lifetime_over_400_days_is_cut_to_400_days() {
    let jar = Jar::new();
    receive(
        &jar,
        "http://h.test/",
        &[
            "l=1; Max-Age=40000000",
            "k=1; Expires=Fri, 01 Jan 2100 00:00:00 GMT",
        ],
        T,
    );
    assert_eq!(
        field(&jar, "http://h.test/", 1_829_865_599).as_deref(),
        Some("l=1; k=1")
    );
    assert_eq!(field(&jar, "http://h.test/", 1_829_865_601), None);
}

#[test]
fn u04_an_expires_that_does_not_parse_is_ignored() {
    let jar = Jar::new();
    receive(&jar, "http://h.test/", &["p=1; Expires=someday"], T);
    assert_eq!(
        field(&jar, "http://h.test/", T + 1_000_000_000).as_deref(),
        Some("p=1")
    );
}

#[test]
fn u05_long_attributes_and_long_cookies_are_ignored() {
    let jar = Jar::new();
    let path = format!("l=1; Path=/{}", "a".repeat(1024));
    receive(&jar, "http://h.test/d/e/set", &[&path], T);
    assert_eq!(
        field(&jar, "http://h.test/d/e/x", T).as_deref(),
        Some("l=1")
    );
    assert_eq!(field(&jar, "http://h.test/", T), None);

    let jar = Jar::new();
    let big = format!("big={}", "x".repeat(4093));
    let huge = format!("huge={}", "y".repeat(4093));
    receive(&jar, "http://h.test/", &[&big, &huge], T);
    assert_eq!(field(&jar, "http://h.test/", T), Some(big));
}

#[test]
fn u06_a_cookie_without_a_name_is_sent_as_its_value() {
    let jar = Jar::new();
    receive(&jar, "http://h.test/", &["abc", "=__Host-x"], T);
    assert_eq!(field(&jar, "http://h.test/", T).as_deref(), Some("abc"));
    assert_eq!(jar.count(at(T)), 1);
}

#[test]
fn u07_a_control_character_ignores_the_cookie() {
    let jar = Jar::new();
    receive(&jar, "http://h.test/", &["a=b\u{1}c"], T);
    assert_eq!(field(&jar, "http://h.test/", T), None);
    assert_eq!(jar.count(at(T)), 0);
}

#[test]
fn u08_prefixes_need_their_attributes() {
    let jar = Jar::new();
    receive(
        &jar,
        "https://h.test/x",
        &[
            "__Secure-a=1; Secure",
            "__Secure-b=1",
            "__Host-d=1; Secure; Path=/",
            "__Host-e=1; Secure; Path=/; Domain=h.test",
            "__Host-f=1; Secure",
            "__HOST-g=1; Secure; Path=/",
        ],
        T,
    );
    receive(&jar, "http://h.test/", &["__secure-c=1; Secure"], T);
    assert_eq!(
        field(&jar, "https://h.test/x", T).as_deref(),
        Some("__Secure-a=1; __Host-d=1; __HOST-g=1")
    );
}

#[test]
fn u09_same_site_by_site_and_method() {
    let jar = Jar::new();
    receive(
        &jar,
        "https://h.test/",
        &[
            "st=1; SameSite=Strict; Secure",
            "lx=1; SameSite=Lax; Secure",
            "df=1; Secure",
            "no=1; SameSite=None; Secure",
            "bd=1; SameSite=Bogus; Secure",
            "ci=1; SameSite=sTrIcT; Secure",
            "cl=1; SameSite=LAX; Secure",
            "nn=1; SameSite=None",
        ],
        T,
    );
    let at_t = |site, method| jar.cookie_field(&url("https://h.test/"), site, method, at(T));
    let all = "st=1; lx=1; df=1; no=1; bd=1; ci=1; cl=1";
    assert_eq!(at_t(Site::Same, Method::Get).as_deref(), Some(all));
    assert_eq!(at_t(Site::Same, Method::Post).as_deref(), Some(all));
    assert_eq!(
        at_t(Site::Cross, Method::Get).as_deref(),
        Some("lx=1; df=1; no=1; bd=1; cl=1")
    );
    assert_eq!(at_t(Site::Cross, Method::Post).as_deref(), Some("no=1"));
}

#[test]
fn u10_same_site_is_same_scheme_and_host() {
    assert_eq!(site(None, &url("http://a.test/")), Site::Same);
    assert_eq!(
        site(Some("http://a.test:1"), &url("http://a.test:2/")),
        Site::Same
    );
    assert_eq!(
        site(Some("http://a.test"), &url("https://a.test/")),
        Site::Cross
    );
    assert_eq!(
        site(Some("http://a.test"), &url("http://b.a.test/")),
        Site::Cross
    );
    assert_eq!(
        site(Some("http://127.0.0.1"), &url("http://127.0.0.2/")),
        Site::Cross
    );
}

#[test]
fn u11_a_replaced_cookie_keeps_its_creation_time() {
    let jar = Jar::new();
    receive(
        &jar,
        "http://h.test/",
        &["a=1; Path=/", "b=1; Path=/", "a=2; Path=/"],
        T,
    );
    assert_eq!(
        field(&jar, "http://h.test/", T).as_deref(),
        Some("a=2; b=1")
    );
}

#[test]
fn u12_the_session_bound_evicts_the_least_recently_used() {
    let jar = Jar::new();
    for host in 1..=60 {
        let address = format!("http://h{host}.test/");
        for n in 1..=50 {
            jar.receive(&url(&address), &format!("c{n}=1"), at(T));
        }
    }
    receive(&jar, "http://h61.test/", &["x=1"], T);
    assert_eq!(jar.count(at(T)), 3000);
    let first = field(&jar, "http://h1.test/", T).expect("h1 keeps cookies");
    assert!(
        first.starts_with("c2=1"),
        "{}",
        &first[..first.len().min(40)]
    );
}

#[test]
fn u13_a_full_domain_evicts_a_cookie_without_secure_first() {
    let jar = Jar::new();
    receive(&jar, "https://h.test/", &["s1=1; Secure"], T);
    for n in 1..=50 {
        jar.receive(&url("https://h.test/"), &format!("n{n}=1"), at(T));
    }
    let sent = field(&jar, "https://h.test/", T).expect("h keeps cookies");
    assert!(
        sent.starts_with("s1=1; n2=1"),
        "{}",
        &sent[..sent.len().min(40)]
    );
    assert_eq!(jar.count(at(T)), 50);
}

#[test]
fn u14_debug_shows_the_count_only() {
    let jar = Jar::new();
    receive(&jar, "http://h.test/", &["s=1"], T);
    assert_eq!(format!("{jar:?}"), "Jar { cookies: 1 }");
    receive(&jar, "http://h.test/", &["m=1; Max-Age=1"], T);
    assert_eq!(jar.count(at(T)), 2);
    assert_eq!(jar.count(at(T + 2)), 1);
}
