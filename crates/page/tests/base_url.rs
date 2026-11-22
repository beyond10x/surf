//! Cases 1 to 3 of `story:agent-follow-links`: the document base URL, from the document's
//! address and its `base` elements.

use surf_html::{ParseOptions, parse_document};
use surf_page::base_url::document_base_url;
use surf_url::Url;

const ADDRESS: &str = "http://127.0.0.1:8000/shop/fruit/index.html?x=1#top";

fn base_of(html: &str) -> String {
    let tree = parse_document(html, &ParseOptions::default());
    let address = Url::parse(ADDRESS, None).expect("the address parses");
    document_base_url(&tree, &address).href()
}

#[test]
fn case_01_no_base_gives_the_document_address() {
    assert_eq!(base_of("<!doctype html><a href=\"a\">a</a>"), ADDRESS);
    // A `base` without `href` counts as none, and so does one inside template contents.
    assert_eq!(
        base_of("<!doctype html><base target=\"_top\"><template><base href=\"/t/\"></template>"),
        ADDRESS
    );
}

#[test]
fn case_02_a_relative_base_href_is_parsed_against_the_document_address() {
    assert_eq!(
        base_of("<!doctype html><head><base href=\"../offers/\"></head><a href=\"a\">a</a>"),
        "http://127.0.0.1:8000/shop/offers/"
    );
    assert_eq!(
        base_of("<!doctype html><base href=\"//other.example/x/\">"),
        "http://other.example/x/"
    );
}

#[test]
fn case_03_of_two_base_elements_the_first_with_href_counts() {
    assert_eq!(
        base_of(
            "<!doctype html><head><base target=\"_blank\"><base href=\"/first/\">\
             <base href=\"/second/\"></head>"
        ),
        "http://127.0.0.1:8000/first/"
    );
    // A first `href` that does not parse gives the document address, not the second.
    assert_eq!(
        base_of("<!doctype html><base href=\"http://[\"><base href=\"/second/\">"),
        ADDRESS
    );
}
