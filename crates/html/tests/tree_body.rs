//! Tree construction outside tables, templates and foreign content: behaviour of the standard
//! that the later tree stories keep.

use surf_html::{NodeData, NodeId, ParseOptions, Tree, parse_document, parse_document_bytes};

/// A node as the tests name it: `document`, an element's local name, or text in quotes.
fn name(tree: &Tree, node: NodeId) -> String {
    match tree.data(node) {
        NodeData::Document => "document".to_string(),
        NodeData::DocumentFragment => "fragment".to_string(),
        NodeData::Doctype { name, .. } => format!("<!DOCTYPE {name}>"),
        NodeData::Element { local_name, .. } => local_name.clone(),
        NodeData::Text(text) => format!("{text:?}"),
        NodeData::Comment(data) => format!("<!--{data}-->"),
        NodeData::ProcessingInstruction { target, .. } => format!("<?{target}>"),
    }
}

fn names(tree: &Tree, nodes: &[NodeId]) -> Vec<String> {
    nodes.iter().map(|&n| name(tree, n)).collect()
}

fn parse(input: &str) -> Tree {
    parse_document(input, &ParseOptions::default())
}

/// The one element named `local_name` under the root.
fn element(tree: &Tree, local_name: &str) -> NodeId {
    let found: Vec<NodeId> = tree
        .descendants(tree.root())
        .into_iter()
        .filter(
            |&n| matches!(tree.data(n), NodeData::Element { local_name: l, .. } if l == local_name),
        )
        .collect();
    assert_eq!(found.len(), 1, "one {local_name}");
    found[0]
}

#[test]
fn node_identities_and_document_order_are_stable() {
    let tree = parse("<p>a<p>b");
    let first = tree.descendants(tree.root());
    let second = tree.descendants(tree.root());
    assert_eq!(first, second);
    assert_eq!(
        names(&tree, &first),
        [
            "document", "html", "head", "body", "p", "\"a\"", "p", "\"b\""
        ]
    );
    for &node in &first[1..] {
        let parent = tree.parent(node).expect("a parent");
        assert!(tree.children(parent).contains(&node));
    }
    assert_eq!(tree.parent(tree.root()), None);
}

#[test]
fn a_frameset_after_an_implied_body_takes_its_place() {
    let tree = parse("<div><frameset>");
    let all = tree.descendants(tree.root());
    assert_eq!(names(&tree, &all), ["document", "html", "head", "frameset"]);
}

#[test]
fn reconstructing_clones_a_formatting_element_after_a_p_end_tag() {
    let tree = parse("<p><b>x</p>y");
    let body = element(&tree, "body");
    let children = tree.children(body);
    assert_eq!(names(&tree, children), ["p", "b"]);
    let b = tree.children(children[0]);
    assert_eq!(names(&tree, b), ["b"]);
    assert_eq!(names(&tree, tree.children(b[0])), ["\"x\""]);
    assert_eq!(names(&tree, tree.children(children[1])), ["\"y\""]);
}

#[test]
fn an_end_tag_of_another_element_stays_text_in_a_title() {
    let tree = parse("<title>a</textarea>b</title>c");
    let title = element(&tree, "title");
    assert_eq!(names(&tree, tree.children(title)), ["\"a</textarea>b\""]);
    let body = element(&tree, "body");
    assert_eq!(names(&tree, tree.children(body)), ["\"c\""]);
}

#[test]
fn a_meta_after_the_prescan_changes_the_encoding_once() {
    let mut bytes = b"<!--".to_vec();
    bytes.resize(2048, b'x');
    bytes.extend_from_slice(b"--><meta charset=\"iso-8859-2\"><meta charset=\"utf-8\">");
    let parsed = parse_document_bytes(&bytes, &ParseOptions::default());
    assert_eq!(parsed.encoding, Some("ISO-8859-2"));
    let head = element(&parsed.tree, "head");
    assert_eq!(
        names(&parsed.tree, parsed.tree.children(head)),
        ["meta", "meta"]
    );
}

#[test]
fn a_third_encoding_named_in_the_re_parse_keeps_the_rest_of_the_document() {
    // The re-parse runs with the confidence certain, so the second `meta` changes nothing and
    // the parse goes on to the end.
    let mut bytes = b"<!--".to_vec();
    bytes.resize(2048, b'x');
    bytes.extend_from_slice(
        b"--><meta charset=\"iso-8859-2\"><meta charset=\"windows-1251\"><p>tail",
    );
    let parsed = parse_document_bytes(&bytes, &ParseOptions::default());
    assert_eq!(parsed.encoding, Some("ISO-8859-2"));
    let p = element(&parsed.tree, "p");
    assert_eq!(names(&parsed.tree, parsed.tree.children(p)), ["\"tail\""]);
}

#[test]
fn a_deep_input_is_parsed_read_and_dropped_without_recursion() {
    // `span` is "any other start tag", which looks at no scope: each block start tag looks for
    // a `p` in button scope over the whole stack, so nested `div` elements take time quadratic
    // in their depth until story `html-parser-limits` bounds the depth.
    let depth = 100_000;
    let tree = parse(&"<span>".repeat(depth));
    let all = tree.descendants(tree.root());
    // The document, `html`, `head`, `body` and the `span` elements.
    assert_eq!(all.len(), depth + 4);
    let deepest = *all.last().expect("a node");
    let mut ancestors = 0;
    let mut node = deepest;
    while let Some(parent) = tree.parent(node) {
        ancestors += 1;
        node = parent;
    }
    assert_eq!(ancestors, depth + 2);
    drop(tree);
}

#[test]
fn a_meta_does_not_override_a_byte_order_mark() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"<meta charset=\"windows-1252\">");
    let parsed = parse_document_bytes(&bytes, &ParseOptions::default());
    assert_eq!(parsed.encoding, Some("UTF-8"));
}
