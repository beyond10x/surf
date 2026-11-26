//! A second adversary's tests of the attribute writes of the tree (`story:scripts-dom-attributes`,
//! after its two fix rounds): what the tree's own documents say of the room of a list the parser
//! built, against the lists the parser builds.

use surf_html::limits::ATTRIBUTE_BYTES;
use surf_html::{NodeData, NodeId, ParseOptions, Tree, parse_document};

/// The first element of `tree` whose local name is `name`.
fn element(tree: &Tree, name: &str) -> NodeId {
    tree.descendants(tree.root())
        .into_iter()
        .find(|&node| matches!(tree.data(node), NodeData::Element { local_name, .. } if local_name == name))
        .expect("the element")
}

/// The length and the room of the attribute list of `node`, in records.
fn list(tree: &Tree, node: NodeId) -> (usize, usize) {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => (attributes.len(), attributes.capacity()),
        _ => panic!("an element"),
    }
}

/// `Tree::attribute_room_grown` says: "A list the parser built has no room, so the first add to
/// a list of `c` records takes `c` records." The parser's merge of a later `html` or `body`
/// start tag ("in body") extends the list by the growth of `Vec`: `<html a1>`, then four names,
/// then a sixth leaves 6 records in room for 10, so the first add of a script takes no record
/// and is charged none for its slot.
#[test]
fn a_list_the_parser_merged_has_room_its_document_denies() {
    for name in ["html", "body"] {
        let page = format!(
            "<{name} a1=\"1\"><{name} a2=\"2\" a3=\"3\" a4=\"4\" a5=\"5\"><{name} a6=\"6\">"
        );
        let tree = parse_document(&page, &ParseOptions::default());
        let node = element(&tree, name);
        let (len, room) = list(&tree, node);
        assert_eq!(len, 6, "{page}");
        assert_eq!(
            tree.attribute_room_grown(node),
            len * ATTRIBUTE_BYTES,
            "{page}: the parser left {len} records in room for {room}; the first add takes {} \
             bytes of record slots, the document says {}",
            tree.attribute_room_grown(node),
            len * ATTRIBUTE_BYTES
        );
    }
}
