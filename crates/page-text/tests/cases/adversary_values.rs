//! Adversary cases of `story:values-and-labels-before-fill`: `look --as tree` and `look --as
//! markdown` print no password value, whatever holds the password input.

use crate::common::parse;

const MARKER: &str = "pw-marker";

#[test]
fn the_outline_and_the_markdown_print_no_password_value() {
    let page = "<!doctype html><title>t</title><form>\
        <label>User <input type=password value=pw-marker></label>\
        <input type=PASSWORD name=p value=pw-marker title=x>\
        <a href=/x>Code <input type=password value=pw-marker></a>\
        <button>Go <input type=password value=pw-marker></button>\
        <p>after</p></form>";
    let tree = parse(page);
    let outline: Vec<String> = surf_page_text::outline(&tree).collect();
    assert!(
        outline.iter().any(|line| line.contains("after")),
        "{outline:?}"
    );
    assert!(
        !outline.iter().any(|line| line.contains(MARKER)),
        "{outline:?}"
    );
    let markdown: Vec<String> = surf_page_text::markdown::markdown(&tree).collect();
    assert!(
        markdown.iter().any(|line| line.contains("after")),
        "{markdown:?}"
    );
    assert!(
        !markdown.iter().any(|line| line.contains(MARKER)),
        "{markdown:?}"
    );
}
