//! Adversarial cases on `story:agent-follow-links`: the document base URL against `base`
//! elements a page can place where the rule does not look.

use surf_html::{ParseOptions, parse_document};
use surf_page::base_url::document_base_url;
use surf_url::Url;

const ADDRESS: &str = "http://127.0.0.1:8000/shop/index.html";

fn base_of(html: &str) -> String {
    let tree = parse_document(html, &ParseOptions::default());
    let address = Url::parse(ADDRESS, None).expect("the address parses");
    document_base_url(&tree, &address).href()
}

/// Only an HTML `base` counts: one in SVG content is an SVG element.
#[test]
fn adversary_follow_a_base_in_svg_content_does_not_count() {
    assert_eq!(
        base_of("<!doctype html><svg><base href=\"http://other.example/\"/></svg><a href=x>x</a>"),
        ADDRESS
    );
}

/// A `base` in the body counts for every link, the ones before it included (the first in tree
/// order); its `href` is stripped of leading and trailing spaces and loses tabs and line feeds.
#[test]
fn adversary_follow_a_base_in_the_body_counts_and_its_href_is_cleaned() {
    assert_eq!(
        base_of("<!doctype html><a href=x>x</a><base href=\"  /o\tff&#10;ers/  \">"),
        "http://127.0.0.1:8000/offers/"
    );
}
