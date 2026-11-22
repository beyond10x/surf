//! A `select` inside a table, read after `story:html-tree-tables`: the table modes put the
//! `select` into its cell, and the `select` start tag rule of in body closes an open `select`
//! instead of nesting a second one.

use surf_html::{NodeData, ParseOptions, parse_document};
use surf_page::{Kind, actionable_elements};

#[test]
fn a_select_in_a_table_is_listed_once_with_kind_select_and_no_value() {
    let tree = parse_document(
        "<!DOCTYPE html><table><tr><td><select id=s><option>a<select id=t><option>b</select>\
         </td></tr></table>",
        &ParseOptions::default(),
    );
    let rows = actionable_elements(&tree);
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row.kind, Kind::Select);
    assert_eq!(row.value, None);
    assert!(!row.hidden);
    let NodeData::Element { attributes, .. } = tree.data(row.node) else {
        panic!("a row that is not an element");
    };
    assert!(
        attributes
            .iter()
            .any(|a| a.local_name == "id" && a.value == "s"),
        "{attributes:?}"
    );
    // The `select` sits in its cell: its parent is a `td`.
    let parent = tree.parent(row.node).expect("a parent");
    assert!(
        matches!(tree.data(parent), NodeData::Element { local_name, .. } if local_name == "td"),
        "{:?}",
        tree.data(parent)
    );
}
