//! Rule R4 of `story:agent-cookie-jar` at its edges: an empty `Domain` attribute, also one that
//! is a dot alone, gives a host-only cookie of the request host (draft-ietf-httpbis-rfc6265bis-22
//! sections 5.6.3 and 5.7 step 10); a cookie of an IPv6 literal host goes to that host alone.
//! Every cookie is a constant of the test.

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

#[test]
fn an_empty_domain_attribute_gives_a_host_only_cookie() {
    for set_cookie in [
        "a=1; Domain=.; Path=/",
        "a=1; Domain=; Path=/",
        "a=1; Domain=example.test; Domain=.; Path=/",
        "a=1; Domain=other.test; Domain=; Path=/",
        "__Host-a=1; Secure; Domain=.; Path=/",
    ] {
        let jar = Jar::new();
        jar.receive(&url("https://h.example.test/"), set_cookie, at(T));
        assert_eq!(jar.count(at(T)), 1, "{set_cookie} was ignored");
        let name = set_cookie.split('=').next().unwrap_or_default();
        assert_eq!(
            field(&jar, "https://h.example.test/x"),
            Some(format!("{name}=1")),
            "{set_cookie}"
        );
        for other in ["https://example.test/", "https://b.h.example.test/"] {
            assert_eq!(field(&jar, other), None, "{set_cookie} reached {other}");
        }
    }
}

#[test]
fn a_cookie_of_an_ipv6_host_goes_to_that_host_alone() {
    let jar = Jar::new();
    jar.receive(&url("http://[::1]:8080/"), "v6=1; Path=/", at(T));
    jar.receive(&url("http://[::2]/"), "v6=1; Domain=[::1]; Path=/", at(T));
    assert_eq!(jar.count(at(T)), 1);
    assert_eq!(field(&jar, "http://[::1]/x").as_deref(), Some("v6=1"));
    assert_eq!(field(&jar, "http://[0:0::1]:9/").as_deref(), Some("v6=1"));
    for other in ["http://[::2]/", "http://127.0.0.1/", "http://localhost/"] {
        assert_eq!(field(&jar, other), None, "{other}");
    }
}
