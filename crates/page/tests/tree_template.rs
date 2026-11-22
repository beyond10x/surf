//! Template contents, read after `story:html-tree-template`: what a `template` holds is parsed
//! into its template contents, outside the document's tree order, so no row comes from it, and
//! the `template` in the head no longer leaves the `body` inside the `head`.

use surf_html::{NodeData, ParseOptions, parse_document};
use surf_page::{Kind, actionable_elements};

#[test]
fn template_contents_are_not_listed() {
    let tree = parse_document(
        "<!DOCTYPE html><head><template><a href=in id=in>in</a><input id=i>\
         <table><tr><td><select id=s></select></td></tr></table></template></head>\
         <body><template><button id=b>b</button></template><a href=out id=out>out</a>",
        &ParseOptions::default(),
    );
    let rows = actionable_elements(&tree);
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row.kind, Kind::Link);
    assert_eq!(row.target.as_deref(), Some("out"));
    assert!(!row.hidden, "{row:?}");
    // The link is in the `body`, a child of the `html` element.
    let body = tree.parent(row.node).expect("a parent");
    assert!(
        matches!(tree.data(body), NodeData::Element { local_name, .. } if local_name == "body"),
        "{:?}",
        tree.data(body)
    );
    let html = tree.parent(body).expect("the html element");
    assert!(
        matches!(tree.data(html), NodeData::Element { local_name, .. } if local_name == "html"),
        "{:?}",
        tree.data(html)
    );
}
