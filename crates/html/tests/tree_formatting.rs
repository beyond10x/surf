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
    // Step 4.9: the common ancestor is the element above the `a`, the outer `div`, and the
    // inner `div` goes there (4.22, 4.23), not into `body`.
    let tree = parse("<div><a><div>x</a>");
    assert_eq!(ancestors_of_text(&tree, "x"), ["a", "div", "div"]);
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
    // 500 `div` keep the stack inside the depth limit (503 open elements).
    assert_eq!(a_elements(500), (9, 513));
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
    let mut node = text_node(tree, text);
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
    // Step 4.4: `</div>` popped the `b`, and `</b>` removes its entry, so the `x` reconstructs
    // nothing.
    let gone = parse("<div><b></div></b>x");
    assert_eq!(ancestors_of_text(&gone, "x"), ["body", "html", "document"]);
    // The scan stops at the last marker: the fourth `b`, after the `marquee`'s marker, removes
    // none of the three before it, and the `x` reconstructs all three.
    let marker = parse("<p><b><b><b><marquee><b></marquee></p>x");
    assert_eq!(elements(&marker, "b").1, 7);
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

#[test]
fn noahs_ark_compares_attributes_in_any_order() {
    // 13.2.4.3, push onto the list of active formatting elements, step 1: two elements have
    // the same attributes when they pair up by name, namespace and value, "the relative order
    // of the attributes does not matter". The fourth `b` finds three identical ones before it
    // and removes the earliest; `</p>` closes all four, and the `x` reconstructs the three left:
    // 7 `b` elements, 8 when the order counted.
    let tree = parse("<p><b id=1 class=c><b class=c id=1><b id=1 class=c><b class=c id=1></p>x");
    assert_eq!(elements(&tree, "b"), (11, 7));
    assert_eq!(ancestors_of_text(&tree, "x"), ["b", "b", "b"]);
}

#[test]
fn a_node_not_in_the_list_leaves_the_stack() {
    // Step 4.18.5: the inner loop meets the `span`, which is not in the list of active
    // formatting elements, and removes it from the stack of open elements. The `div` moves to
    // `body` (4.22, 4.23), the new `a` into the `div` (4.24 to 4.30), and the second outer
    // iteration pops that `a` (4.8). `</div>` leaves `body` the current node, so the `z` is a
    // child of `body`; with the `span` left on the stack the `z` would go into the `span`.
    let tree = parse("<a><span><div>x</a></div>z");
    assert_eq!(ancestors_of_text(&tree, "x"), ["a", "div", "body"]);
    assert_eq!(ancestors_of_text(&tree, "z"), ["body", "html", "document"]);
}

#[test]
fn an_inner_loop_element_is_created_for_the_token_of_its_node() {
    // Step 4.18.6: the new element for the `b` is created "for the token for which the element
    // node was created", so it has the `id` of `<b id=1>`. It takes the `div` (4.18.11), goes
    // into `body` (4.22, 4.23), and the new `a` goes into the `div` (4.24 to 4.30).
    let tree = parse("<a><b id=1><div>x</a>");
    let mut node = text_node(&tree, "x");
    for _ in 0..3 {
        node = tree.parent(node).expect("a parent");
    }
    assert_eq!(name(&tree, node), "b");
    assert_eq!(
        attributes(&tree, node),
        [("id".to_string(), "1".to_string())]
    );
    assert_eq!(ancestors_of_text(&tree, "x"), ["a", "div", "b"]);
    // Step 4.24: the new `a` is created for the token the `a` was created for.
    let tree = parse("<a href=h><div>x</a>");
    let a = tree.parent(text_node(&tree, "x")).expect("a parent");
    assert_eq!(
        attributes(&tree, a),
        [("href".to_string(), "h".to_string())]
    );
}

/// The one text node `text`.
fn text_node(tree: &Tree, text: &str) -> NodeId {
    let quoted = format!("{text:?}");
    all(tree)
        .into_iter()
        .find(|&n| name(tree, n) == quoted)
        .expect("the text")
}

/// The attributes of the element `node`, as name and value.
fn attributes(tree: &Tree, node: NodeId) -> Vec<(String, String)> {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes
            .iter()
            .map(|a| (a.local_name.clone(), a.value.clone()))
            .collect(),
        _ => panic!("an element"),
    }
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
    // Step 2 of the `a` rule reconstructs the `b` that `</p>` closed, and the `a` goes into it.
    let tree = parse("<p><b></p><a>x");
    assert_eq!(ancestors_of_text(&tree, "x"), ["a", "b", "body"]);
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
    // The `nobr` rule reconstructs first, with no `nobr` in scope: the `b` that `</p>` closed is
    // created again, and the `nobr` goes into it.
    let tree = parse("<p><b></p><nobr>x");
    assert_eq!(ancestors_of_text(&tree, "x"), ["nobr", "b", "body"]);
}
