//! Rule R6 of `story:agent-cookie-jar`, draft-ietf-httpbis-rfc6265bis-22 section 5.7 step 16: a
//! cookie without `Secure` from an `http:` response is ignored when a `Secure` cookie of the same
//! name and domain is stored whose path the new cookie's path path-matches. The draft's note to
//! that step: given a `Secure` `a` on `/login`, an `http:` response may set `a` on `/` or `/foo`,
//! not on `/login` or `/login/en`. Every cookie is a constant of the test.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use surf_cookie::{Jar, Method, Site};
use surf_url::Url;

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

/// A jar holding `a=1; Secure; Path=/login` of `h.test`.
fn secure_on_login() -> Jar {
    let jar = Jar::new();
    jar.receive(&url("https://h.test/login"), "a=1; Secure; Path=/login", at(T));
    jar
}

#[test]
fn an_http_cookie_on_the_secure_cookies_path_or_below_it_is_ignored() {
    for (from, set_cookie) in [
        ("http://h.test/login", "a=2; Path=/login"),
        ("http://h.test/login/x", "a=2; Path=/login/en"),
        ("http://h.test/login/en/x", "a=2"),
    ] {
        let jar = secure_on_login();
        jar.receive(&url(from), set_cookie, at(T));
        assert_eq!(jar.count(at(T)), 1, "{set_cookie} from {from} was stored");
        assert_eq!(
            field(&jar, "https://h.test/login/en/page").as_deref(),
            Some("a=1"),
            "{set_cookie} from {from}"
        );
    }
}

#[test]
fn an_http_cookie_beside_or_above_the_secure_cookies_path_is_stored() {
    for (from, set_cookie, sent_to) in [
        ("http://h.test/", "a=2; Path=/", "http://h.test/"),
        ("http://h.test/foo/x", "a=2; Path=/foo", "http://h.test/foo"),
        ("http://h.test/x", "a=2; Path=/log", "http://h.test/log"),
    ] {
        let jar = secure_on_login();
        jar.receive(&url(from), set_cookie, at(T));
        assert_eq!(jar.count(at(T)), 2, "{set_cookie} from {from} was ignored");
        assert_eq!(
            field(&jar, sent_to).as_deref(),
            Some("a=2"),
            "{set_cookie} from {from}"
        );
    }
}

#[test]
fn an_https_response_may_replace_the_secure_cookie() {
    let jar = secure_on_login();
    jar.receive(&url("https://h.test/login"), "a=3; Path=/login", at(T));
    assert_eq!(jar.count(at(T)), 1);
    assert_eq!(field(&jar, "http://h.test/login").as_deref(), Some("a=3"));
    jar.receive(&url("https://h.test/login/x"), "a=4; Path=/login/en", at(T));
    assert_eq!(
        field(&jar, "https://h.test/login/en").as_deref(),
        Some("a=4; a=3")
    );
}
