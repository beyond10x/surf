//! Rule R13 of `story:agent-cookie-jar`, the memory of one jar: the bytes its cookies store
//! (names, values, paths and domains) stay at most `STORED_BYTES_MAX`, a path counted as stored,
//! however long the addresses the cookies came from; over the bound the cookie with the earliest
//! last access goes first. The draft's minimums still fit. Every cookie is a constant of the test.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use surf_cookie::{COOKIES_MAX, Jar, Method, STORED_BYTES_MAX, Site};
use surf_url::Url;

const T: u64 = 1_795_305_600;

fn url(address: &str) -> Url {
    Url::parse(address, None).expect("a test address parses")
}

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

#[test]
fn cookies_without_a_path_from_long_addresses_stay_within_the_bound() {
    let jar = Jar::new();
    // The default-path of each address is 60,001 bytes: `/` and 60,000 `p`.
    let path = format!("/{}", "p".repeat(60_000));
    let value = "v".repeat(4_000);
    let mut received = 0usize;
    let mut last = String::new();
    for host in 1..=6 {
        let from = url(&format!("http://h{host}.test{path}/x"));
        for n in 1..=50 {
            jar.receive(&from, &format!("c{n}={value}"), at(T));
            received += 1;
            let stored = jar.stored_bytes();
            assert!(
                stored <= STORED_BYTES_MAX,
                "after {received} cookies the jar stores {stored} bytes"
            );
        }
        last = format!("http://h{host}.test{path}/y");
    }
    // 300 cookies of about 64 KiB each do not all fit: the earliest went first.
    let count = jar.count(at(T));
    assert!(count < 300, "{count} cookies");
    assert!(
        jar.stored_bytes() > STORED_BYTES_MAX - 70_000,
        "the jar evicted more than it needed: {} bytes",
        jar.stored_bytes()
    );
    let field = jar
        .cookie_field(&url(&last), Site::Same, Method::Get, at(T))
        .expect("the last host's cookies are kept");
    assert!(field.starts_with("c1="), "the last host's first cookie");
    assert_eq!(
        field.matches("; ").count(),
        1,
        "8,192 bytes hold two cookies"
    );
    assert!(
        !jar.cookie_field(
            &url(&format!("http://h1.test{path}/y")),
            Site::Same,
            Method::Get,
            at(T)
        )
        .is_some_and(|field| field.starts_with("c1=")),
        "the earliest cookie of the first host went first"
    );
}

#[test]
fn the_drafts_minimums_fit_within_the_bound() {
    let jar = Jar::new();
    // A `Path` attribute of 1,024 bytes; a host of 253 bytes, the longest DNS name.
    let path = format!("/{}", "q".repeat(1_023));
    let labels = vec!["b".repeat(50); 4].join(".");
    for host in 0..COOKIES_MAX / 50 {
        let name = format!("h{host:02}{}.{labels}", "a".repeat(46));
        assert_eq!(name.len(), 253);
        let from = url(&format!("https://{name}/"));
        for n in 0..50 {
            let cookie_name = format!("c{n:02}");
            let value = "v".repeat(4_096 - cookie_name.len());
            jar.receive(&from, &format!("{cookie_name}={value}; Path={path}"), at(T));
        }
    }
    assert_eq!(jar.count(at(T)), COOKIES_MAX);
    assert_eq!(jar.stored_bytes(), 3_000 * (4_096 + 1_024 + 253));
    assert!(jar.stored_bytes() <= STORED_BYTES_MAX);
}
