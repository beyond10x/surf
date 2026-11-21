//! Misnested formatting elements: the adoption agency algorithm, its two bounds, the Noah's Ark
//! clause, and the `a` and `nobr` start tags that run the algorithm (story
//! `html-tree-formatting`). The tests hold node counts, never a time.

use surf_html::{NodeData, NodeId, ParseOptions, Tree, parse_document};

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

/// The nodes reached from the root.
fn all(tree: &Tree) -> Vec<NodeId> {
    tree.descendants(tree.root())
}

/// The number of elements reached from the root, and of those named `local_name`.
fn elements(tree: &Tree, local_name: &str) -> (usize, usize) {
    let mut every = 0;
    let mut named = 0;
    for node in all(tree) {
        if let NodeData::Element { local_name: l, .. } = tree.data(node) {
            every += 1;
            if l == local_name {
                named += 1;
            }
        }
    }
    (every, named)
}

/// The one `body` element.
fn body_of(tree: &Tree) -> NodeId {
    let found: Vec<NodeId> = all(tree)
        .into_iter()
        .filter(|&n| matches!(tree.data(n), NodeData::Element { local_name, .. } if local_name == "body"))
        .collect();
    assert_eq!(found.len(), 1, "one body");
    found[0]
}

#[test]
fn the_inner_loop_clones_at_most_three_elements() {
    // The inner loop meets `em`, `u` and `s` and clones them; `i` and `b` come after the third
    // iteration and leave the list, so they are not cloned.
    let tree = parse("<a><b><i><s><u><em><div>x</a>");
    assert_eq!(elements(&tree, "a"), (14, 2));
    // The `div` keeps its place under the clone of `em`; its children go to the new `a`.
    assert_eq!(ancestors_of_text(&tree, "x"), ["a", "div", "em"]);
}

#[test]
fn the_outer_loop_runs_at_most_eight_times() {
    let a_elements = |n: usize| {
        let tree = parse(&format!("<a>{}</a>", "<div>".repeat(n)));
        (elements(&tree, "a").1, all(&tree).len())
    };
    assert_eq!(a_elements(7).0, 8);
    for n in [8, 9, 20] {
        assert_eq!(a_elements(n).0, 9, "n = {n}");
    }
    assert_eq!(a_elements(1_000), (9, 1_013));
    // With nine `div` the loop stops with the last new `a` still in the list. The first outer
    // iteration moved the bookmark after the new `b`, so that `a` is after the `b` in the list:
    // once the `div` elements are closed, the `y` goes into an `a` created inside the `b`.
    let tree = parse(&format!(
        "<a><b>{}</a>{}y",
        "<div>".repeat(9),
        "</div>".repeat(9)
    ));
    assert_eq!(ancestors_of_text(&tree, "y"), ["a", "b", "body"]);
}

/// The names of the parent, grandparent and great-grandparent of the one text node `text`.
fn ancestors_of_text(tree: &Tree, text: &str) -> Vec<String> {
    let quoted = format!("{text:?}");
    let mut node = all(tree)
        .into_iter()
        .find(|&n| name(tree, n) == quoted)
        .expect("the text");
    let mut out = Vec::new();
    for _ in 0..3 {
        node = tree.parent(node).expect("a parent");
        out.push(name(tree, node));
    }
    out
}

#[test]
fn noahs_ark_keeps_three_identical_formatting_elements() {
    // `</p>` closes the 100 `b`; the `x` reconstructs the entries left after the last marker.
    let same = parse(&format!("<p>{}</p>x", "<b>".repeat(100)));
    assert_eq!(elements(&same, "b").1, 103);
    assert_eq!(all(&same).len(), 109);
    let different: String = (0..100).map(|k| format!("<b id={k}>")).collect();
    let different = parse(&format!("<p>{different}</p>x"));
    assert_eq!(elements(&different, "b").1, 200);
    assert_eq!(all(&different).len(), 206);
    // The `b` the fourth one took out of the list is closed by "any other end tag", which the
    // algorithm falls back to when no `b` is in the list.
    let closed = parse("<b><b><b><b></b></b></b><span></b>x");
    assert_eq!(
        ancestors_of_text(&closed, "x"),
        ["body", "html", "document"]
    );
    // The fourth `b` removes the earliest of the three before it, the one before the `i`: the
    // reconstruction after `</p>` creates `i` first.
    let earliest = parse("<p><b><i><b><b><b></p>x");
    let body = body_of(&earliest);
    let mut chain = Vec::new();
    let mut node = last_child(&earliest, body);
    while let Some(next) = node {
        chain.push(name(&earliest, next));
        node = last_child(&earliest, next);
    }
    assert_eq!(chain, ["i", "b", "b", "b", "\"x\""]);
}

/// The last child of `node`, if any.
fn last_child(tree: &Tree, node: NodeId) -> Option<NodeId> {
    tree.children(node).last().copied()
}

#[test]
fn an_a_start_tag_closes_an_open_a() {
    let tree = parse("<a>1<a>2");
    let body = body_of(&tree);
    let children = tree.children(body);
    assert_eq!(names(&tree, children), ["a", "a"]);
    assert_eq!(names(&tree, tree.children(children[0])), ["\"1\""]);
    assert_eq!(names(&tree, tree.children(children[1])), ["\"2\""]);
    assert_eq!(all(&tree).len(), 8);
    // The standard's example: the first `a` is not in scope inside the table, so the algorithm
    // returns early, and the `a` rule itself takes it out of the list and the stack. The `x`
    // goes into the one `a` the reconstruction creates, a child of `body`.
    let tree = parse("<a href=a>a<table><a href=b>b</table>x");
    assert_eq!(elements(&tree, "a").1, 3);
    assert_eq!(ancestors_of_text(&tree, "x"), ["a", "body", "html"]);
    // An `a` before the last marker is not looked at: it stays open, and the `y` goes into it.
    let tree = parse("<a><marquee><a>x</marquee>y");
    assert_eq!(ancestors_of_text(&tree, "y"), ["a", "body", "html"]);
}

#[test]
fn a_nobr_start_tag_closes_an_open_nobr() {
    let tree = parse("<nobr>1<nobr>2");
    let body = body_of(&tree);
    let children = tree.children(body);
    assert_eq!(names(&tree, children), ["nobr", "nobr"]);
    assert_eq!(names(&tree, tree.children(children[0])), ["\"1\""]);
    assert_eq!(names(&tree, tree.children(children[1])), ["\"2\""]);
    assert_eq!(all(&tree).len(), 8);
    // The algorithm closes the `b` with the first `nobr`; the reconstruction after it creates
    // a `b` again before the second `nobr` is inserted, so that `nobr` is inside it.
    let tree = parse("<nobr><b>1<nobr>2");
    let body = body_of(&tree);
    let children = tree.children(body);
    assert_eq!(names(&tree, children), ["nobr", "b"]);
    assert_eq!(names(&tree, tree.children(children[1])), ["nobr"]);
}
