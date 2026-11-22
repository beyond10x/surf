//! Adversarial cases against the page listing after the tree stories of round 1: the brief's
//! question whether a table, template or `select` input lists an element twice.

use surf_html::{ParseOptions, parse_document};
use surf_page::{Kind, actionable_elements};

/// `story:html-tree-complete` copies the selected `option`'s children into each
/// `selectedcontent` of its `select` at the end of the parse (`tree_builder/selectedcontent.rs`).
/// A link inside the option is then two elements, and `actionable_elements` lists both as
/// visible rows with the same target: the agent sees one link of the page twice.
#[test]
fn a_link_in_the_selected_option_is_listed_once() {
    let tree = parse_document(
        "<!DOCTYPE html><select><button><selectedcontent></selectedcontent></button>\
         <option><a href=x>x</a></option></select>",
        &ParseOptions::default(),
    );
    let rows = actionable_elements(&tree);
    let links: Vec<_> = rows.iter().filter(|r| r.kind == Kind::Link).collect();
    assert_eq!(links.len(), 1, "{links:?}");
}
