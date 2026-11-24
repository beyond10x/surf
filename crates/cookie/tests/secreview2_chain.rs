//! Security review, second pass, of `story:agent-cookie-jar`: [`Chain`] called directly. R9
//! with draft 22 section 5.2: once a redirect chain has been on another site, every later
//! request is cross-site. "Same site" is the same scheme and the same host, ports ignored
//! (decision 2), so a change of scheme alone leaves the site.

use surf_cookie::{Chain, Site};
use surf_url::Url;

fn url(address: &str) -> Url {
    Url::parse(address, None).expect("a test address parses")
}

/// What `Chain::site` answers for each address of one chain, in order.
fn sites(initiator: Option<&str>, addresses: &[&str]) -> Vec<Site> {
    let mut chain = Chain::new();
    addresses
        .iter()
        .map(|address| chain.site(initiator, &url(address)))
        .collect()
}

#[test]
fn a_change_of_scheme_alone_leaves_the_site() {
    assert_eq!(
        sites(
            None,
            &["http://h.test/", "https://h.test/x", "https://h.test/y"]
        ),
        [Site::Same, Site::Cross, Site::Cross]
    );
}

#[test]
fn ports_alone_do_not_leave_the_site() {
    assert_eq!(
        sites(
            None,
            &["http://h.test:1/", "http://h.test:2/", "http://h.test/"]
        ),
        [Site::Same, Site::Same, Site::Same]
    );
}

#[test]
fn a_chain_that_leaves_and_comes_back_twice_stays_cross_site() {
    assert_eq!(
        sites(
            Some("http://v.test"),
            &[
                "http://v.test/a",
                "http://o.test/b",
                "http://v.test/c",
                "http://o.test/d",
                "http://v.test/e",
            ]
        ),
        [
            Site::Same,
            Site::Cross,
            Site::Cross,
            Site::Cross,
            Site::Cross
        ]
    );
}

#[test]
fn a_chain_that_starts_on_another_site_never_becomes_same_site() {
    assert_eq!(
        sites(
            Some("http://v.test"),
            &["http://o.test/a", "http://v.test/b", "http://v.test/c"]
        ),
        [Site::Cross, Site::Cross, Site::Cross]
    );
}
