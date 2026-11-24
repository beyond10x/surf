//! The section "IDL Index" of a standard's page: the text content of each `pre` element after
//! the `h2` element whose `id` is `idl-index` and before the next `h2` element, in tree order.

use surf_html::{NodeData, NodeId, ParseOptions, Tree};

use crate::ReadError;

/// The text of the IDL Index of the page `bytes`, parsed with `surf-html`: each `pre`
/// element's text content, each ending in a line feed (one is added where it does not).
pub fn extract_index(bytes: &[u8], file: &str) -> Result<String, ReadError> {
    let parsed = surf_html::parse_document_bytes(bytes, &ParseOptions::default());
    let tree = &parsed.tree;
    if let Some(limit) = tree.stopped_by() {
        return Err(ReadError::new(
            file,
            0,
            format!("a page the HTML parser stopped at its limit {limit:?}"),
        ));
    }
    let order = tree.descendants(tree.root());
    let Some(heading) = order.iter().position(|&node| {
        element_name(tree, node) == Some("h2") && attribute(tree, node, "id") == Some("idl-index")
    }) else {
        return Err(ReadError::new(
            file,
            0,
            "a page without an `h2` with the id `idl-index`",
        ));
    };
    let mut text = String::new();
    let mut at = heading + tree.descendants(order[heading]).len();
    while at < order.len() {
        let node = order[at];
        match element_name(tree, node) {
            Some("h2") => break,
            Some("pre") => {
                let inside = tree.descendants(node);
                for &descendant in &inside {
                    if let NodeData::Text(data) = tree.data(descendant) {
                        text.push_str(data);
                    }
                }
                if !text.ends_with('\n') {
                    text.push('\n');
                }
                at += inside.len();
            }
            _ => at += 1,
        }
    }
    if text.is_empty() {
        return Err(ReadError::new(
            file,
            0,
            "an IDL Index without a `pre` element",
        ));
    }
    Ok(text)
}

/// The local name of `node` when it is an element of the HTML namespace.
fn element_name(tree: &Tree, node: NodeId) -> Option<&str> {
    match tree.data(node) {
        NodeData::Element {
            namespace: surf_html::Namespace::Html,
            local_name,
            ..
        } => Some(local_name),
        _ => None,
    }
}

fn attribute<'a>(tree: &'a Tree, node: NodeId, name: &str) -> Option<&'a str> {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes
            .iter()
            .find(|attribute| attribute.namespace.is_none() && attribute.local_name == name)
            .map(|attribute| attribute.value.as_str()),
        _ => None,
    }
}
