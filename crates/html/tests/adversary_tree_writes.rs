//! An adversary's tests of the attribute reads and writes of the tree's public surface
//! (`story:scripts-dom-attributes`): the position rule against a plain model of the DOM's
//! "get an attribute by name", and the tree's own count against a fresh count after each write.

use surf_html::{Attribute, Namespace, NodeData, NodeId, ParseOptions, Tree, parse_document};

/// The page of these tests: an HTML element with two attributes, an SVG element whose name the
/// parser keeps in mixed case and an SVG `use` with an `xlink:href`.
const PAGE: &str = "<!DOCTYPE html><html><body><p id=\"a\" data-x=\"1\">t</p>\
<svg id=\"g\" viewBox=\"0 0 1 1\"><use id=\"u\" xlink:href=\"#a\"></use></svg></body></html>";

fn parsed() -> Tree {
    parse_document(PAGE, &ParseOptions::default())
}

/// The element whose attribute `id` in no namespace is `id`.
fn by_id(tree: &Tree, id: &str) -> NodeId {
    tree.descendants(tree.root())
        .into_iter()
        .find(|&node| match tree.data(node) {
            NodeData::Element { attributes, .. } => attributes
                .iter()
                .any(|a| a.namespace.is_none() && a.local_name == "id" && a.value == id),
            _ => false,
        })
        .expect("the element")
}

fn list(tree: &Tree, node: NodeId) -> &Vec<Attribute> {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes,
        _ => panic!("an element"),
    }
}

/// The DOM's "get an attribute by name", written plainly: the name lowered for an element in
/// the HTML namespace, then the first attribute whose qualified name equals it.
fn model(tree: &Tree, node: NodeId, name: &str) -> Option<usize> {
    let NodeData::Element {
        namespace,
        attributes,
        ..
    } = tree.data(node)
    else {
        return None;
    };
    let wanted = if *namespace == Namespace::Html {
        name.to_ascii_lowercase()
    } else {
        name.to_string()
    };
    attributes.iter().position(|a| {
        let qualified = match &a.prefix {
            Some(prefix) => format!("{prefix}:{}", a.local_name),
            None => a.local_name.clone(),
        };
        qualified == wanted
    })
}

/// The stored bytes of `tree` counted afresh from the nodes under its root.
fn fresh_count(tree: &Tree) -> usize {
    tree.descendants(tree.root())
        .into_iter()
        .map(|node| match tree.data(node) {
            NodeData::Document | NodeData::DocumentFragment => 0,
            NodeData::Doctype {
                name,
                public_id,
                system_id,
            } => name.len() + public_id.len() + system_id.len(),
            NodeData::Element {
                local_name,
                attributes,
                ..
            } => {
                local_name.len()
                    + attributes
                        .iter()
                        .map(Attribute::stored_bytes)
                        .sum::<usize>()
            }
            NodeData::Text(text) => text.len(),
            NodeData::Comment(data) => data.len(),
            NodeData::ProcessingInstruction { target, data } => target.len() + data.len(),
        })
        .sum()
}

/// A wanted name that is a prefix of a stored qualified name, or longer than one with the
/// stored name as its prefix, or empty, matches nothing, by the reads and by the writes.
#[test]
fn a_shorter_or_longer_name_matches_nothing() {
    let mut tree = parsed();
    let p = by_id(&tree, "a");
    let u = by_id(&tree, "u");
    for name in ["", "i", "I", "idx", "data", "data-x-y", "d"] {
        assert_eq!(tree.attribute_position(p, name), None, "{name:?} on p");
        assert_eq!(
            tree.attribute_position_ns(p, None, name),
            None,
            "{name:?} on p"
        );
    }
    for name in ["", "xlink", "xlink:", "xlink:h", "xlink:hrefs", "i"] {
        assert_eq!(tree.attribute_position(u, name), None, "{name:?} on use");
    }
    tree.remove_attribute(p, "i").unwrap();
    tree.remove_attribute(p, "").unwrap();
    assert_eq!(tree.toggle_attribute(p, "data", Some(false)), Ok(false));
    assert_eq!(list(&tree, p).len(), 2, "nothing removed");
    tree.set_attribute(p, "i", "v").unwrap();
    assert_eq!(list(&tree, p).len(), 3, "a new attribute");
    assert_eq!(list(&tree, p)[0].value, "a", "the id kept its value");
}

/// After each write of every kind, the tree's total equals a fresh count, a list holds room for
/// at most twice its records (or four), the position rule answers what the plain model answers
/// for every probe name, and `attribute_names` answers each name in its qualified length.
#[test]
fn each_write_keeps_the_count_the_room_and_the_rule() {
    let mut tree = parsed();
    let p = by_id(&tree, "a");
    let g = by_id(&tree, "g");
    let u = by_id(&tree, "u");
    assert_eq!(fresh_count(&tree), tree.stored_bytes(), "the parse");
    let xlink = Some(Namespace::XLink);
    let probes = [
        "id",
        "ID",
        "data-x",
        "Data-X",
        "viewBox",
        "viewbox",
        "ViewBox",
        "xlink:href",
        "XLINK:HREF",
        "l:href",
        "a:b",
        "A:B",
        "foo",
        "FOO",
        "hidden",
        "",
        "x",
    ];
    let mut writes: Vec<(&str, Box<dyn Fn(&mut Tree)>)> = vec![
        (
            "set ID on p",
            Box::new(move |t: &mut Tree| t.set_attribute(p, "ID", "z").unwrap()),
        ),
        (
            "set FOO in no namespace",
            Box::new(move |t: &mut Tree| t.set_attribute_ns(p, None, "FOO", "1").unwrap()),
        ),
        (
            "set a:b on p",
            Box::new(move |t: &mut Tree| t.set_attribute(p, "A:B", "1").unwrap()),
        ),
        (
            "set a:b in XLink",
            Box::new(move |t: &mut Tree| t.set_attribute_ns(p, xlink, "a:b", "2").unwrap()),
        ),
        (
            "set ViewBox on svg",
            Box::new(move |t: &mut Tree| t.set_attribute(g, "ViewBox", "2").unwrap()),
        ),
        (
            "set l:href on use",
            Box::new(move |t: &mut Tree| t.set_attribute_ns(u, xlink, "l:href", "#c").unwrap()),
        ),
        (
            "toggle hidden on",
            Box::new(move |t: &mut Tree| assert!(t.toggle_attribute(p, "HIDDEN", None).unwrap())),
        ),
        (
            "set a long value",
            Box::new(move |t: &mut Tree| t.set_attribute(p, "data-x", &"v".repeat(1_000)).unwrap()),
        ),
        (
            "set a short value",
            Box::new(move |t: &mut Tree| t.set_attribute(p, "DATA-X", "w").unwrap()),
        ),
        (
            "remove a:b",
            Box::new(move |t: &mut Tree| t.remove_attribute(p, "a:b").unwrap()),
        ),
        (
            "toggle hidden off",
            Box::new(move |t: &mut Tree| assert!(!t.toggle_attribute(p, "hidden", None).unwrap())),
        ),
        (
            "remove FOO",
            Box::new(move |t: &mut Tree| t.remove_attribute_ns(p, None, "FOO").unwrap()),
        ),
        (
            "remove xlink:href",
            Box::new(move |t: &mut Tree| t.remove_attribute(u, "xlink:href").unwrap()),
        ),
        (
            "remove ViewBox",
            Box::new(move |t: &mut Tree| t.remove_attribute(g, "ViewBox").unwrap()),
        ),
    ];
    for n in 0..8 {
        let name: &'static str = Box::leak(format!("n{n}").into_boxed_str());
        writes.push((
            name,
            Box::new(move |t: &mut Tree| t.set_attribute(p, name, "").unwrap()),
        ));
    }
    for n in 0..8 {
        let name: &'static str = Box::leak(format!("n{n}").into_boxed_str());
        writes.push((
            name,
            Box::new(move |t: &mut Tree| t.remove_attribute(p, name).unwrap()),
        ));
    }
    for (what, write) in &writes {
        write(&mut tree);
        assert_eq!(fresh_count(&tree), tree.stored_bytes(), "{what}");
        for node in [p, g, u] {
            let attributes = list(&tree, node);
            assert!(
                attributes.capacity() <= (2 * attributes.len()).max(4),
                "{what}: {} records, room for {}",
                attributes.len(),
                attributes.capacity()
            );
            for name in probes {
                assert_eq!(
                    tree.attribute_position(node, name),
                    model(&tree, node, name),
                    "{what}: {name:?}"
                );
            }
            let names = tree.attribute_names(node);
            assert_eq!(names.len(), attributes.len(), "{what}");
            for (name, attribute) in names.iter().zip(attributes) {
                assert_eq!(name.len(), attribute.qualified_len(), "{what}: {name}");
            }
        }
    }
}
