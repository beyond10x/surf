//! What a tree keeps beside its nodes (story `scripts-dom-reading`): the document's mode, which
//! the parser writes where the initial insertion mode decides it ([`Tree::quirks_mode`]), and
//! the count of the writes to its child lists ([`Tree::writes`]), which a reader of a child list
//! compares to know whether one changed. The mode is part of what a tree holds; the count is
//! not.

use surf_html::limits::Limits;
use surf_html::tree::WriteResult;
use surf_html::{
    Context, Namespace, NodeData, NodeId, ParseOptions, QuirksMode, Tree, parse_document,
    parse_fragment,
};

fn parse(input: &str) -> Tree {
    parse_document(input, &ParseOptions::default())
}

/// The doctypes of the parser's own test of the mode, and a page without one.
#[test]
fn the_tree_holds_the_mode_the_doctype_decides() {
    let cases = [
        ("<p>", QuirksMode::Quirks),
        (
            "<!DOCTYPE html PUBLIC \"-//W3O//DTD W3 HTML Strict 3.0//EN//\"><p>",
            QuirksMode::Quirks,
        ),
        (
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \
             \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\"><p>",
            QuirksMode::LimitedQuirks,
        ),
        ("<!DOCTYPE html><p>", QuirksMode::NoQuirks),
    ];
    for (input, mode) in cases {
        assert_eq!(parse(input).quirks_mode(), mode, "{input}");
    }
    let context = Context {
        namespace: Namespace::Html,
        local_name: "body".to_string(),
    };
    let fragment = parse_fragment("<p>x", &context, &ParseOptions::default());
    assert_eq!(fragment.quirks_mode(), QuirksMode::NoQuirks);
}

/// The page of the write cases: a `div` with a `p`, a text, a comment, and an `svg`.
const PAGE: &str = "<!DOCTYPE html><html><head></head><body>\
                    <div class=c><p>one</p>two<!--c--></div><svg></svg></body></html>";

/// The nodes of [`PAGE`] the cases write.
struct Nodes {
    body: NodeId,
    div: NodeId,
    p: NodeId,
    two: NodeId,
    comment: NodeId,
    svg: NodeId,
}

fn nodes(tree: &Tree) -> Nodes {
    let all = tree.descendants(tree.root());
    let element = |name: &str| {
        all.iter()
            .copied()
            .find(|&n| matches!(tree.data(n), NodeData::Element { local_name, .. } if local_name == name))
            .expect("the element")
    };
    let find = |test: &dyn Fn(&NodeData) -> bool| {
        all.iter()
            .copied()
            .find(|&n| test(tree.data(n)))
            .expect("the node")
    };
    Nodes {
        body: element("body"),
        div: element("div"),
        p: element("p"),
        two: find(&|data| matches!(data, NodeData::Text(text) if text == "two")),
        comment: find(&|data| matches!(data, NodeData::Comment(_))),
        svg: element("svg"),
    }
}

/// A write case: its name, whether the write raises the count, what it does first without
/// being looked at, and the write.
type Case = (
    &'static str,
    bool,
    fn(&mut Tree, &Nodes) -> WriteResult<()>,
    fn(&mut Tree, &Nodes) -> WriteResult<()>,
);

/// The write changes a child list: the count is raised.
const RAISED: bool = true;

/// The write changes no child list (it adds a node to no list, or changes a node's data or
/// attributes): the count is left as it was.
const LEFT: bool = false;

fn nothing(_: &mut Tree, _: &Nodes) -> WriteResult<()> {
    Ok(())
}

/// The cases: one per function of the public write surface that changes a child list or does
/// not, and one per side for a function that does either.
const WRITES: &[Case] = &[
    ("create_element_ns", LEFT, nothing, |t, _| {
        t.create_element_ns(Namespace::Html, "b").map(drop)
    }),
    ("create_text_node", LEFT, nothing, |t, _| {
        t.create_text_node("x").map(drop)
    }),
    ("create_comment", LEFT, nothing, |t, _| {
        t.create_comment("x").map(drop)
    }),
    ("create_processing_instruction", LEFT, nothing, |t, _| {
        t.create_processing_instruction("x", "y").map(drop)
    }),
    ("create_document_fragment", LEFT, nothing, |t, _| {
        t.create_document_fragment().map(drop)
    }),
    ("insert_before", RAISED, nothing, |t, n| {
        t.insert_before(n.div, n.two, Some(n.p))
    }),
    ("insert_before", LEFT, nothing, |t, n| {
        t.insert_before(n.div, n.p, Some(n.two))
    }),
    ("append_child", RAISED, nothing, |t, n| {
        t.append_child(n.body, n.p)
    }),
    ("append_child", LEFT, nothing, |t, n| {
        t.append_child(n.div, n.comment)
    }),
    ("remove_node", RAISED, nothing, |t, n| t.remove_node(n.p)),
    (
        "remove_node",
        LEFT,
        |t, n| t.remove_node(n.p),
        |t, n| t.remove_node(n.p),
    ),
    ("replace_child", RAISED, nothing, |t, n| {
        t.replace_child(n.div, n.comment, n.p)
    }),
    ("replace_child", LEFT, nothing, |t, n| {
        t.replace_child(n.div, n.p, n.p)
    }),
    ("move_before", RAISED, nothing, |t, n| {
        t.move_before(n.div, n.comment, Some(n.p))
    }),
    ("move_before", LEFT, nothing, |t, n| {
        t.move_before(n.div, n.p, Some(n.two))
    }),
    ("clone_node", RAISED, nothing, |t, n| {
        t.clone_node(n.p, true).map(drop)
    }),
    ("clone_node", LEFT, nothing, |t, n| {
        t.clone_node(n.two, true).map(drop)
    }),
    ("import_node", RAISED, nothing, |t, n| {
        t.import_node(n.p, true).map(drop)
    }),
    ("import_node", LEFT, nothing, |t, n| {
        t.import_node(n.two, true).map(drop)
    }),
    ("adopt_node", RAISED, nothing, |t, n| t.adopt_node(n.p)),
    (
        "adopt_node",
        LEFT,
        |t, n| t.remove_node(n.p),
        |t, n| t.adopt_node(n.p),
    ),
    (
        "normalize",
        RAISED,
        |t, n| t.split_text(n.two, 1).map(drop),
        |t, n| t.normalize(n.div),
    ),
    ("normalize", LEFT, nothing, |t, n| t.normalize(n.div)),
    ("split_text", RAISED, nothing, |t, n| {
        t.split_text(n.two, 1).map(drop)
    }),
    (
        "split_text",
        LEFT,
        |t, n| t.remove_node(n.two),
        |t, n| t.split_text(n.two, 1).map(drop),
    ),
    ("replace_data", LEFT, nothing, |t, n| {
        t.replace_data(n.two, 0, 1, "x")
    }),
    ("set_data", LEFT, nothing, |t, n| t.set_data(n.two, "x")),
    ("append_data", LEFT, nothing, |t, n| {
        t.append_data(n.two, "x")
    }),
    ("insert_data", LEFT, nothing, |t, n| {
        t.insert_data(n.two, 1, "x")
    }),
    ("delete_data", LEFT, nothing, |t, n| {
        t.delete_data(n.two, 0, 1)
    }),
    ("set_text_content", RAISED, nothing, |t, n| {
        t.set_text_content(n.div, "")
    }),
    ("set_text_content", RAISED, nothing, |t, n| {
        t.set_text_content(n.svg, "x")
    }),
    ("set_text_content", LEFT, nothing, |t, n| {
        t.set_text_content(n.svg, "")
    }),
    ("set_text_content", LEFT, nothing, |t, n| {
        t.set_text_content(n.two, "x")
    }),
    ("set_text_content", LEFT, nothing, |t, _| {
        let document = t.root();
        t.set_text_content(document, "x")
    }),
    ("set_attribute", LEFT, nothing, |t, n| {
        t.set_attribute(n.div, "title", "x")
    }),
    ("set_attribute_ns", LEFT, nothing, |t, n| {
        t.set_attribute_ns(n.svg, Some(Namespace::XLink), "xlink:href", "x")
    }),
    ("remove_attribute", LEFT, nothing, |t, n| {
        t.remove_attribute(n.div, "class")
    }),
    ("remove_attribute_ns", LEFT, nothing, |t, n| {
        t.remove_attribute_ns(n.div, None, "class")
    }),
    ("toggle_attribute", LEFT, nothing, |t, n| {
        t.toggle_attribute(n.div, "hidden", None).map(drop)
    }),
];

/// A parse raises the count, and so does each successful write of the public surface that
/// changes a child list; every other write leaves it as it was.
#[test]
fn a_write_to_a_child_list_raises_the_count() {
    assert!(parse("<p>").writes() > 0, "a parse writes");
    let mut names = Vec::new();
    for (name, raised, prepare, write) in WRITES {
        let mut tree = parse(PAGE);
        let nodes = nodes(&tree);
        prepare(&mut tree, &nodes).expect(name);
        let before = tree.writes();
        write(&mut tree, &nodes).expect(name);
        assert_eq!(
            tree.writes() != before,
            *raised,
            "{name}: raised {raised} expected, the count went from {before} to {}",
            tree.writes()
        );
        names.push(*name);
    }
    names.dedup();
    assert_eq!(
        names.len(),
        26,
        "one function of the surface each: {names:?}"
    );
}

/// The count is not part of what a tree holds; the mode is. Two parses of one DOCTYPE token
/// under a node limit of 1 hold the document alone (the doctype is refused, the parse stops):
/// with the same name they are equal, with another name their modes differ and nothing else.
#[test]
fn the_count_is_not_compared_and_the_mode_is() {
    let a = parse(PAGE);
    let mut b = a.clone();
    let Nodes { div, p, two, .. } = nodes(&b);
    b.remove_node(p).expect("remove");
    b.insert_before(div, p, Some(two)).expect("put back");
    assert_ne!(a.writes(), b.writes());
    assert_eq!(a, b, "trees that differ only in their counts");

    let options = ParseOptions {
        limits: Limits {
            max_nodes: 1,
            ..Limits::default()
        },
        ..ParseOptions::default()
    };
    let html = parse_document("<!DOCTYPE html>", &options);
    let again = parse_document("<!DOCTYPE html>", &options);
    let other = parse_document("<!DOCTYPE htmx>", &options);
    assert_eq!(html.node_count(), 1);
    assert_eq!(other.node_count(), 1);
    assert_eq!(html, again);
    assert_eq!(
        (html.quirks_mode(), other.quirks_mode()),
        (QuirksMode::NoQuirks, QuirksMode::Quirks)
    );
    assert_ne!(html, other, "trees that differ only in their modes");
}
