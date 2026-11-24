//! Adversarial cases of the second review of `story:html-script-hook`, against fix round 1:
//! the refusal of a parser insertion that would make a cycle, the depth check behind "a hook
//! wrote" (`Tree::reparented`), one call for each script element the parser inserts, and the
//! attribute lookups after a write. Each test names what it holds the unit to: the rulings of
//! the round, `docs/design/scripts.md` Q1 and the documentation of `src/limits.rs`.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use surf_html::limits::{Limit, Limits};
use surf_html::{
    Namespace, NodeData, NodeId, NotRun, ParseOptions, ScriptHook, Tree, parse_document,
    parse_document_with_hook,
};

/// One call of a hook: the element, and `None` for `run` or the reason of `skip`.
type Call = (NodeId, Option<NotRun>);

/// What a recording hook does in `run` after it recorded the call.
type Action = Box<dyn FnMut(&mut Tree, NodeId)>;

/// A hook that records each call, and runs an action in `run`.
#[derive(Default)]
struct Recorder {
    calls: Vec<Call>,
    on_run: Option<Action>,
}

impl Recorder {
    fn with(action: impl FnMut(&mut Tree, NodeId) + 'static) -> Recorder {
        Recorder {
            calls: Vec::new(),
            on_run: Some(Box::new(action)),
        }
    }
}

impl ScriptHook for Recorder {
    fn run(&mut self, tree: &mut Tree, script: NodeId) {
        self.calls.push((script, None));
        if let Some(action) = self.on_run.as_mut() {
            action(tree, script);
        }
    }

    fn skip(&mut self, _: &Tree, script: NodeId, reason: NotRun) {
        self.calls.push((script, Some(reason)));
    }
}

fn scripted() -> ParseOptions {
    ParseOptions {
        scripting_disabled: false,
        ..ParseOptions::default()
    }
}

fn scripted_with(limits: Limits) -> ParseOptions {
    ParseOptions {
        scripting_disabled: false,
        limits,
    }
}

fn deep(max_depth: usize) -> ParseOptions {
    scripted_with(Limits {
        max_depth,
        ..Limits::default()
    })
}

fn attribute<'t>(tree: &'t Tree, node: NodeId, name: &str) -> Option<&'t str> {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes
            .iter()
            .find(|a| a.namespace.is_none() && a.local_name == name)
            .map(|a| a.value.as_str()),
        _ => None,
    }
}

fn id_of(tree: &Tree, node: NodeId) -> String {
    attribute(tree, node, "id").unwrap_or_default().to_string()
}

fn named(tree: &Tree, calls: &[Call]) -> Vec<(String, Option<NotRun>)> {
    calls.iter().map(|&(n, c)| (id_of(tree, n), c)).collect()
}

fn is_element(tree: &Tree, node: NodeId) -> bool {
    matches!(tree.data(node), NodeData::Element { .. })
}

fn contents_of(tree: &Tree, template: NodeId) -> Option<NodeId> {
    match tree.data(template) {
        NodeData::Element {
            template_contents: Some(contents),
            ..
        } => Some(*contents),
        _ => None,
    }
}

/// An HTML or SVG `script` element: a script element (a MathML `script` is none).
fn is_script_element(tree: &Tree, node: NodeId) -> bool {
    matches!(tree.data(node), NodeData::Element {
        namespace: Namespace::Html | Namespace::Svg,
        local_name,
        ..
    } if local_name == "script")
}

/// Every node reached from `starts` through children, template contents and parents, each
/// once: a cycle does not make the walk loop.
fn reach(tree: &Tree, starts: &[NodeId]) -> BTreeSet<NodeId> {
    let mut seen = BTreeSet::new();
    let mut pending = starts.to_vec();
    while let Some(node) = pending.pop() {
        if !seen.insert(node) {
            continue;
        }
        pending.extend(tree.children(node).iter().copied());
        pending.extend(contents_of(tree, node));
        pending.extend(tree.parent(node));
    }
    seen
}

/// Every node reached from the root through children and template contents, each once.
fn connected(tree: &Tree) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut pending = vec![tree.root()];
    while let Some(node) = pending.pop() {
        if !seen.insert(node) {
            continue;
        }
        out.push(node);
        pending.extend(contents_of(tree, node));
        pending.extend(tree.children(node).iter().rev());
    }
    out
}

fn by_id(tree: &Tree, id: &str) -> NodeId {
    connected(tree)
        .into_iter()
        .find(|&n| attribute(tree, n, "id") == Some(id))
        .unwrap_or_else(|| panic!("no #{id} in the tree"))
}

fn by_name(tree: &Tree, local_name: &str) -> NodeId {
    connected(tree)
        .into_iter()
        .find(
            |&n| matches!(tree.data(n), NodeData::Element { local_name: l, .. } if l == local_name),
        )
        .unwrap_or_else(|| panic!("no {local_name}"))
}

/// The most elements on a path down from the root, a template counted for the nodes of its
/// contents.
fn max_element_depth(tree: &Tree) -> usize {
    let mut most = 0;
    let mut seen = HashSet::new();
    let mut pending = vec![(tree.root(), 0usize)];
    while let Some((node, above)) = pending.pop() {
        if !seen.insert(node) {
            continue;
        }
        let depth = above + usize::from(is_element(tree, node));
        most = most.max(depth);
        pending.extend(tree.children(node).iter().map(|&c| (c, depth)));
        if let Some(contents) = contents_of(tree, node) {
            pending.push((contents, depth));
        }
    }
    most
}

/// What keeps a tree from being a tree, among the nodes reached from the root and from `known`:
/// a child whose parent is another node, a child listed twice, a node listed by a parent it
/// does not name, a document that is not the root, a template's contents with a parent, and a
/// cycle of parents and template hosts. Bounded by the arena.
fn faults(tree: &Tree, known: &[NodeId]) -> Vec<String> {
    let bound = tree.node_count();
    let root = tree.root();
    let mut found = Vec::new();
    let mut seen: HashSet<NodeId> = HashSet::new();
    let mut hosts: HashMap<NodeId, NodeId> = HashMap::new();
    let mut pending: Vec<NodeId> = known.to_vec();
    pending.push(root);
    while let Some(node) = pending.pop() {
        if !seen.insert(node) {
            continue;
        }
        if seen.len() > bound {
            found.push("more nodes reached than the arena holds".to_string());
            break;
        }
        let mut listed = HashSet::new();
        for &child in tree.children(node) {
            if !listed.insert(child) {
                found.push(format!("{child:?} listed twice by {node:?}"));
            }
            if tree.parent(child) != Some(node) {
                found.push(format!(
                    "{child:?} is listed by {node:?} but names {:?} as its parent",
                    tree.parent(child)
                ));
            }
            pending.push(child);
        }
        if let Some(contents) = contents_of(tree, node) {
            hosts.insert(contents, node);
            if tree.parent(contents).is_some() {
                found.push(format!("the contents {contents:?} have a parent"));
            }
            pending.push(contents);
        }
        if let Some(parent) = tree.parent(node) {
            if !tree.children(parent).contains(&node) {
                found.push(format!("{node:?} names {parent:?}, which does not list it"));
            }
            pending.push(parent);
        }
        if node != root && *tree.data(node) == NodeData::Document {
            found.push(format!("{node:?} is a document and not the root"));
        }
    }
    if tree.parent(root).is_some() {
        found.push("the root has a parent".to_string());
    }
    for &node in &seen {
        let mut at = node;
        let mut steps = 0;
        while let Some(up) = tree.parent(at).or_else(|| hosts.get(&at).copied()) {
            at = up;
            steps += 1;
            if steps > bound {
                found.push(format!(
                    "{node:?} is on a cycle of parents and template hosts"
                ));
                break;
            }
        }
    }
    found
}

fn assert_a_tree(tree: &Tree, known: &[NodeId], what: &str) {
    let found = faults(tree, known);
    assert!(found.is_empty(), "{what}: not a tree: {found:#?}");
}

/// A chain of `length` new HTML `div` elements, each the only child of the one before: the
/// first and the last.
fn chain(tree: &mut Tree, length: usize) -> (NodeId, NodeId) {
    let first = tree.create_element_ns(Namespace::Html, "div").unwrap();
    let mut last = first;
    for _ in 1..length {
        let next = tree.create_element_ns(Namespace::Html, "div").unwrap();
        tree.append_child(last, next).unwrap();
        last = next;
    }
    (first, last)
}

// The cases below held on the tip they were written against (`72de3b9`).

// Item 1 of the coordinator's message: a parser insertion the tree refuses.

/// The hook takes the furthest block out and puts the common ancestor inside it; `</b>` then
/// would insert the furthest block under its own descendant (step 4.23), which is not made.
/// What "the parse goes on" is: the furthest block stays without a parent and on the stack of
/// open elements; the new `b` of step 4.30 goes under it with its children; the second
/// iteration of the outer loop pops that `b` (step 4.8), so later text, elements and a script
/// go under the furthest block, out of the document; the script is reported `disconnected`,
/// once; `</body>`, `</html>` and a comment after them go on as without the hook; every node
/// is counted; the caller can write the detached part back.
#[test]
fn a_refused_parser_insertion_leaves_a_detached_subtree_and_the_parse_goes_on() {
    let input = "<body><div id=ca><b id=fmt><p id=fb><script id=s>x</script></b>after\
                 <i id=late>l</i><script id=s2>y</script></body></html><!--c-->";
    let kept = Rc::new(RefCell::new(Vec::new()));
    let keep = Rc::clone(&kept);
    let mut hook = Recorder::with(move |tree, script| {
        let fb = tree.parent(script).unwrap();
        let fmt = tree.parent(fb).unwrap();
        let ca = tree.parent(fmt).unwrap();
        tree.remove_node(fb).unwrap();
        tree.append_child(fb, ca).unwrap();
        keep.borrow_mut().extend([fb, fmt, ca, script]);
    });
    let mut tree = parse_document_with_hook(input, &scripted(), &mut hook);
    let [fb, fmt, ca, s] = <[NodeId; 4]>::try_from(kept.borrow().clone()).unwrap();
    assert_a_tree(&tree, &[fb, fmt, ca, s], input);
    assert_eq!(
        tree.parent(fb),
        None,
        "the refused furthest block has no parent"
    );
    let new_b = tree.children(fb)[0];
    assert_eq!(
        tree.children(fb).len(),
        4,
        "the new b, `after`, `i` and `s2`"
    );
    assert!(matches!(tree.data(new_b), NodeData::Element { local_name, .. } if local_name == "b"));
    assert_ne!(new_b, fmt);
    assert_eq!(tree.parent(ca), Some(new_b));
    assert_eq!(tree.parent(s), Some(new_b));
    let detached = reach(&tree, &[fb]);
    let in_document: BTreeSet<NodeId> = connected(&tree).into_iter().collect();
    assert!(detached.is_disjoint(&in_document));
    let texts: Vec<&str> = detached
        .iter()
        .filter_map(|&n| match tree.data(n) {
            NodeData::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        texts.contains(&"after") && texts.contains(&"l"),
        "{texts:?}"
    );
    let s2 = *detached
        .iter()
        .find(|&&n| attribute(&tree, n, "id") == Some("s2"))
        .expect("s2 is under the furthest block");
    assert_eq!(tree.parent(s2), Some(fb));
    assert_eq!(hook.calls, [(s, None), (s2, Some(NotRun::Disconnected))]);
    let body = by_name(&tree, "body");
    assert!(tree.children(body).is_empty());
    let root = tree.root();
    assert_eq!(
        tree.data(*tree.children(root).last().unwrap()),
        &NodeData::Comment("c".to_string())
    );
    assert_eq!(tree.stopped_by(), None);
    assert!(tree.node_count() >= in_document.len() + detached.len());
    tree.append_child(body, fb).unwrap();
    assert_a_tree(&tree, &[], "the detached part written back");
    assert!(connected(&tree).contains(&s2));
    assert_eq!(
        tree.append_child(ca, fb),
        Err(surf_html::WriteError::HierarchyRequest)
    );
}

/// Each way into an element the parser inserts after a write that gave a node a new parent
/// (`insert_element`, the in-head `script` start tag, the `template` start tag, a foreign
/// element) stops at the limit: at a parent N − 1 deep the element is inserted, N deep; at a
/// parent N deep it is not and the parse names `Depth`. The parent is put at its depth by the
/// hook, so the stack of open elements alone does not tell it.
#[test]
fn after_a_write_each_way_into_an_element_stops_at_the_depth_limit() {
    const N: usize = 8;
    let mut wrong = Vec::new();
    for tag in ["p", "script", "template", "svg", "table", "select"] {
        for parent_depth in [N - 1, N] {
            let input = format!("<body><div id=d><script id=s>x</script><{tag} id=t>");
            let mut hook = Recorder::with(move |tree, script| {
                let d = tree.parent(script).unwrap();
                let html = tree.parent(tree.parent(d).unwrap()).unwrap();
                tree.remove_node(script).unwrap();
                let (first, last) = chain(tree, parent_depth - 2);
                tree.append_child(html, first).unwrap();
                tree.append_child(last, d).unwrap();
            });
            let tree = parse_document_with_hook(&input, &deep(N), &mut hook);
            assert_a_tree(&tree, &[], &input);
            let inserted = connected(&tree)
                .into_iter()
                .any(|n| attribute(&tree, n, "id") == Some("t"));
            let expected = parent_depth < N;
            let depth = max_element_depth(&tree);
            let stopped = tree.stopped_by();
            if inserted != expected || depth > N || (stopped == Some(Limit::Depth)) == expected {
                wrong.push(format!(
                    "<{tag}> under a parent {parent_depth} deep: inserted {inserted}, depth \
                     {depth}, stopped by {stopped:?}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "limit {N}: {wrong:#?}");
}

// Item 4 of the coordinator's message: the switch "a hook wrote".

/// A write that moves a node to where it already is turns the switch on and changes nothing:
/// the tree equals the tree of the same input parsed with no hook, the switch included in no
/// comparison; and so does a tree whose hook wrote only to an element no tree holds, but for
/// the nodes it made.
#[test]
fn a_write_that_changes_nothing_leaves_the_tree_equal() {
    let input = "<p>a<script id=s>x</script><b>c<i>d</i></b><table><tr><td>e</table>";
    let without = parse_document(input, &scripted());
    let mut hook = Recorder::with(|tree, script| {
        let text = tree.children(script)[0];
        tree.append_child(script, text).unwrap();
    });
    let tree = parse_document_with_hook(input, &scripted(), &mut hook);
    assert_eq!(tree, without);
    let mut hook = Recorder::with(|tree, script| {
        let p = tree.parent(script).unwrap();
        tree.insert_before(p, script, None).unwrap();
    });
    let tree = parse_document_with_hook(input, &scripted(), &mut hook);
    assert_eq!(tree, without);
}

/// Markup that makes a tree deeper than its stack of open elements with no hook (found by a
/// random search on `main`'s parser, at a depth limit of 16: 18 elements deep).
const DEEPER_THAN_THE_STACK: &str = "<script id=s>x</script><form><button><u></i></form><u><u>\
    <option></a><dd><code></form><i><ul></p><option><td><optgroup><s></div><em></template></u>\
    <form><div><i><td></table><h1></i><option></form><table><template><form><object>\
    <font color=red></html>";

/// Refused writes give no node a new parent and leave the switch off: the markup after them,
/// which makes a tree deeper than the stack, parses as without a hook.
#[test]
fn a_refused_write_leaves_the_parse_as_without_a_hook() {
    const N: usize = 16;
    let without = parse_document(DEEPER_THAN_THE_STACK, &deep(N));
    assert!(max_element_depth(&without) > N);
    let mut hook = Recorder::with(|tree, script| {
        let head = tree.parent(script).unwrap();
        let html = tree.parent(head).unwrap();
        let root = tree.root();
        assert!(tree.append_child(script, html).is_err());
        assert!(tree.append_child(script, root).is_err());
        assert!(tree.insert_before(head, script, Some(html)).is_err());
        assert!(tree.replace_child(script, head, html).is_err());
        assert!(tree.move_before(script, html, None).is_err());
    });
    let tree = parse_document_with_hook(DEEPER_THAN_THE_STACK, &deep(N), &mut hook);
    assert_eq!(tree, without);
    assert_eq!(tree.stopped_by(), without.stopped_by());
}

/// The names the `html` and `body` merges keep are read again after a `run`: an attribute the
/// hook removed from the `body` is added by a later `<body>` start tag, and one it added is
/// not replaced.
#[test]
fn a_body_merge_after_a_hook_reads_the_attributes_the_hook_left() {
    let input = "<body a=1><body c=1><script id=s>x</script><body a=2 b=2 z=2>";
    let mut hook = Recorder::with(|tree, script| {
        let body = tree.parent(script).unwrap();
        tree.remove_attribute(body, "a").unwrap();
        tree.set_attribute(body, "b", "hook").unwrap();
        tree.set_attribute_ns(body, Some(Namespace::XLink), "xlink:z", "ns")
            .unwrap();
    });
    let tree = parse_document_with_hook(input, &scripted(), &mut hook);
    let body = by_name(&tree, "body");
    assert_eq!(attribute(&tree, body, "a"), Some("2"));
    assert_eq!(attribute(&tree, body, "b"), Some("hook"));
    assert_eq!(attribute(&tree, body, "c"), Some("1"));
    assert_eq!(attribute(&tree, body, "z"), Some("2"));
    let NodeData::Element { attributes, .. } = tree.data(body) else {
        unreachable!()
    };
    assert_eq!(attributes.len(), 5, "{attributes:?}");
}

// Item 3 of the coordinator's message: one call for each script element the parser inserts.

/// The ways a script element leaves the stack that the first review did not list, and the
/// scripts a hook moves, removes, clones or makes: each element the parser inserted gets one
/// call, in the order the elements left the stack; a clone, a script the hook made and a
/// MathML `script` get none.
#[test]
fn every_script_the_parser_inserts_gets_one_call_whatever_a_hook_does() {
    use NotRun::*;
    type Case = (
        &'static str,
        fn(&mut Tree, NodeId),
        &'static [(&'static str, Option<NotRun>)],
    );
    fn nothing(_: &mut Tree, _: NodeId) {}
    fn remove_outer(tree: &mut Tree, _: NodeId) {
        let o = by_id(tree, "o");
        tree.remove_node(o).unwrap();
    }
    fn outer_into_template(tree: &mut Tree, _: NodeId) {
        let o = by_id(tree, "o");
        let template = tree.create_element_ns(Namespace::Html, "template").unwrap();
        let body = by_name(tree, "body");
        tree.append_child(body, template).unwrap();
        let contents = contents_of(tree, template).unwrap();
        tree.append_child(contents, o).unwrap();
    }
    fn clone_outer(tree: &mut Tree, _: NodeId) {
        let o = by_id(tree, "o");
        let copy = tree.clone_node(o, true).unwrap();
        let body = by_name(tree, "body");
        tree.append_child(body, copy).unwrap();
    }
    fn make_scripts(tree: &mut Tree, script: NodeId) {
        let parent = tree.parent(script).unwrap();
        for namespace in [Namespace::Html, Namespace::Svg] {
            let made = tree.create_element_ns(namespace, "script").unwrap();
            let text = tree.create_text_node("made").unwrap();
            tree.append_child(made, text).unwrap();
            tree.append_child(parent, made).unwrap();
        }
    }
    fn move_itself_back(tree: &mut Tree, script: NodeId) {
        let parent = tree.parent(script).unwrap();
        tree.remove_node(script).unwrap();
        tree.append_child(parent, script).unwrap();
    }
    let open_svg = "<svg><script id=o><foreignObject><script id=i>x</script></foreignObject>\
                    </script></svg><script id=after>z</script>";
    let cases: &[Case] = &[
        (
            open_svg,
            nothing,
            &[("i", None), ("o", Some(Svg)), ("after", None)],
        ),
        (
            open_svg,
            remove_outer,
            &[("i", None), ("o", Some(Svg)), ("after", None)],
        ),
        (
            open_svg,
            outer_into_template,
            &[("i", None), ("o", Some(Svg)), ("after", None)],
        ),
        (
            open_svg,
            clone_outer,
            &[("i", None), ("o", Some(Svg)), ("after", None)],
        ),
        (
            "<script id=a>x</script><script id=b>y</script>",
            make_scripts,
            &[("a", None), ("b", None)],
        ),
        (
            "<script id=a>x</script><script id=b>y</script>",
            move_itself_back,
            &[("a", None), ("b", None)],
        ),
        (
            // `foreignObject` bounds the scope: `</b>` is ignored and `o` stays open.
            "<b><svg><script id=o><foreignObject><p>x</b><script id=a>y</script>",
            nothing,
            &[("a", None), ("o", Some(Svg))],
        ),
        (
            "<b><svg><script id=o></b><script id=a>y</script>",
            nothing,
            &[("o", Some(Svg)), ("a", None)],
        ),
        (
            "<table><tr><td><svg><script id=o></table><script id=a>y</script>",
            nothing,
            &[("o", Some(Svg)), ("a", None)],
        ),
        (
            "<template><svg><script id=o></template><script id=a>y</script>",
            nothing,
            &[("o", Some(Svg)), ("a", None)],
        ),
        (
            "<select><option><svg><script id=o></select><script id=a>y</script>",
            nothing,
            &[("o", Some(Svg)), ("a", None)],
        ),
        (
            "<math><annotation-xml><script id=m>x</script></annotation-xml>\
             <mi><svg><script id=o>v</script></svg></mi></math>",
            nothing,
            &[("o", Some(Svg))],
        ),
        (
            "<svg><script id=o><svg><script id=p></svg></svg>tail",
            nothing,
            &[("p", Some(Svg)), ("o", Some(Svg))],
        ),
        ("<frameset><svg><script id=o>", nothing, &[]),
        (
            // `</body>` pops nothing: `a` is an SVG `script` inside `o`.
            "<body><svg><script id=o></body><script id=a>y</script>",
            nothing,
            &[("a", Some(Svg)), ("o", Some(Svg))],
        ),
    ];
    let mut wrong = Vec::new();
    for &(input, action, expected) in cases {
        // The action runs at the first `run` only.
        let mut first = true;
        let mut hook = Recorder::with(move |tree, script| {
            if std::mem::take(&mut first) {
                action(tree, script);
            }
        });
        let tree = parse_document_with_hook(input, &scripted(), &mut hook);
        let calls = hook.calls;
        let known: Vec<NodeId> = calls.iter().map(|c| c.0).collect();
        assert_a_tree(&tree, &known, input);
        let got = named(&tree, &calls);
        let expected: Vec<(String, Option<NotRun>)> =
            expected.iter().map(|&(i, r)| (i.to_string(), r)).collect();
        let unique: HashSet<NodeId> = known.iter().copied().collect();
        if got != expected || unique.len() != known.len() {
            wrong.push(format!("`{input}`: calls {got:?}, expected {expected:?}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// A hook that parses another document with a hook of its own: each parse calls its own hook,
/// once for each of its scripts, and the outer parse goes on as before.
#[test]
fn a_parse_inside_a_hook_calls_only_its_own_hook() {
    let inner_calls = Rc::new(RefCell::new(Vec::new()));
    let inner = Rc::clone(&inner_calls);
    let mut hook = Recorder::with(move |_, _| {
        let mut nested = Recorder::default();
        let tree = parse_document_with_hook(
            "<script id=n>y</script><svg><script id=m /></svg><script id=e>z",
            &scripted(),
            &mut nested,
        );
        inner.borrow_mut().push(named(&tree, &nested.calls));
    });
    let tree = parse_document_with_hook(
        "<script id=a>x</script><script id=b>y</script>",
        &scripted(),
        &mut hook,
    );
    assert_eq!(
        named(&tree, &hook.calls),
        [("a".to_string(), None), ("b".to_string(), None)]
    );
    let once = vec![
        ("n".to_string(), None),
        ("m".to_string(), Some(NotRun::Svg)),
        ("e".to_string(), Some(NotRun::Eof)),
    ];
    assert_eq!(*inner_calls.borrow(), [once.clone(), once]);
}

// Items 1 and 3: random writes and markup.

/// A small xorshift generator: the same seed gives the same case.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// The markup of the random cases: misnested formatting, tables, templates, `select`,
/// foreign content and every kind of script.
const PIECES: &[&str] = &[
    "<b>",
    "</b>",
    "<i>",
    "</i>",
    "<a>",
    "</a>",
    "<nobr>",
    "<p>",
    "</p>",
    "<div>",
    "</div>",
    "<span>",
    "<table>",
    "</table>",
    "<tr>",
    "<td>",
    "</td>",
    "<caption>",
    "<template>",
    "</template>",
    "<select>",
    "</select>",
    "<option>",
    "<button>",
    "<selectedcontent>",
    "<svg>",
    "</svg>",
    "<math>",
    "<mi>",
    "<foreignObject>",
    "</foreignObject>",
    "<font color=red>",
    "<li>",
    "<h1>",
    "<form>",
    "</form>",
    "<em>",
    "x",
    "</body>",
    "</html>",
    "<body x=1>",
    "<html y=2>",
    "<object>",
    "<pre>",
    "<br>",
    "</p>",
    "<annotation-xml encoding=text/html>",
    "<script>s</script>",
    "<script>s</script>",
    "<script>s</script>",
    "<script>s</script>",
    "<script type=text/plain>t</script>",
    "<svg><script>v</script>",
    "<svg><script/>",
    "<svg><script>",
    "<svg><script><foreignObject><script>w</script>",
    "<math><mi><script>m</script>",
    "<math><script>q</script>",
    "<template><script>t</script></template>",
    "<table><script>u</script>",
];

/// A write whose answer the random hook does not read: a refused write is part of the case.
fn tried<T>(_: Result<T, surf_html::WriteError>) {}

/// A hook that writes at random on the nodes it has seen and made.
struct Random {
    rng: Rng,
    reparenting: bool,
    calls: Vec<Call>,
    seen: BTreeSet<NodeId>,
    made: BTreeSet<NodeId>,
}

impl Random {
    fn write(&mut self, tree: &mut Tree, pool: &[NodeId]) {
        let pick = |rng: &mut Rng| pool[rng.below(pool.len())];
        let a = pick(&mut self.rng);
        let b = pick(&mut self.rng);
        let c = pick(&mut self.rng);
        let op = if self.reparenting {
            self.rng.below(16)
        } else {
            [2, 7, 8, 9, 10, 11, 12][self.rng.below(7)]
        };
        match op {
            0 => tried(tree.append_child(a, b)),
            1 => tried(tree.insert_before(a, b, Some(c))),
            2 => tried(tree.remove_node(a)),
            3 => tried(tree.replace_child(a, b, c)),
            4 => tried(tree.move_before(a, b, Some(c))),
            5 => {
                let names = [
                    (Namespace::Html, "div"),
                    (Namespace::Html, "template"),
                    (Namespace::Html, "script"),
                    (Namespace::Svg, "script"),
                    (Namespace::Html, "b"),
                    (Namespace::Html, "table"),
                    (Namespace::Html, "p"),
                    (Namespace::Html, "select"),
                    (Namespace::MathMl, "annotation-xml"),
                    (Namespace::Svg, "foreignObject"),
                ];
                let (namespace, name) = names[self.rng.below(names.len())];
                if let Ok(new) = tree.create_element_ns(namespace, name) {
                    self.made.extend(reach(tree, &[new]));
                    tried(tree.append_child(a, new));
                }
            }
            6 => {
                if let Ok(copy) = tree.clone_node(a, self.rng.below(2) == 0) {
                    self.made.extend(reach(tree, &[copy]));
                    tried(tree.append_child(b, copy));
                }
            }
            7 => tried(tree.normalize(a)),
            8 => tried(tree.adopt_node(a)),
            9 => tried(tree.set_attribute(a, "encoding", "text/html")),
            10 => tried(tree.set_attribute_ns(a, Some(Namespace::XLink), "xlink:type", "x")),
            11 => tried(tree.remove_attribute(a, "encoding")),
            12 => tried(tree.set_data(a, "")),
            13 => tried(tree.set_text_content(a, "t")),
            14 => {
                if let Ok(fragment) = tree.create_document_fragment() {
                    self.made.insert(fragment);
                    tried(tree.append_child(fragment, a));
                    tried(tree.append_child(fragment, b));
                    tried(tree.insert_before(c, fragment, None));
                }
            }
            _ => {
                if let Ok(rest) = tree.split_text(a, 1) {
                    self.made.insert(rest);
                }
            }
        }
    }
}

impl ScriptHook for Random {
    fn run(&mut self, tree: &mut Tree, script: NodeId) {
        self.calls.push((script, None));
        self.seen.insert(script);
        self.seen.extend(connected(tree));
        let pool: Vec<NodeId> = self.seen.union(&self.made).copied().collect();
        for _ in 0..1 + self.rng.below(4) {
            self.write(tree, &pool);
        }
    }

    fn skip(&mut self, _: &Tree, script: NodeId, reason: NotRun) {
        self.calls.push((script, Some(reason)));
        self.seen.insert(script);
    }
}

/// One random case: what is wrong with its tree and its calls, if anything.
fn random_case(
    seed: u64,
    reparenting: bool,
    options: &ParseOptions,
) -> (String, Vec<String>, usize) {
    let mut rng = Rng::new(seed);
    let mut input = String::new();
    for _ in 0..4 + rng.below(36) {
        input.push_str(PIECES[rng.below(PIECES.len())]);
    }
    let mut hook = Random {
        rng,
        reparenting,
        calls: Vec::new(),
        seen: BTreeSet::new(),
        made: BTreeSet::new(),
    };
    let tree = parse_document_with_hook(&input, options, &mut hook);
    let mut wrong = Vec::new();
    let known: Vec<NodeId> = hook.seen.union(&hook.made).copied().collect();
    wrong.extend(faults(&tree, &known));
    let mut called = HashSet::new();
    for &(node, _) in &hook.calls {
        if !called.insert(node) {
            wrong.push(format!("{node:?} called twice"));
        }
        if !is_script_element(&tree, node) || hook.made.contains(&node) {
            wrong.push(format!(
                "{node:?} called and no script element the parser made"
            ));
        }
    }
    let mut starts = known.clone();
    starts.push(tree.root());
    for node in reach(&tree, &starts) {
        if is_script_element(&tree, node) && !hook.made.contains(&node) && !called.contains(&node) {
            wrong.push(format!("{node:?} no call"));
        }
    }
    let runs = hook.calls.iter().filter(|c| c.1.is_none()).count();
    (input, wrong, runs)
}

/// Runs `cases` random cases on a thread and fails when a case is wrong or panics, when one
/// takes more than 5 s, or when fewer than `cases` hooks ran.
fn random_cases(cases: u64, reparenting: bool, options: ParseOptions) {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        for seed in 0..cases {
            let result = std::panic::catch_unwind(|| random_case(seed, reparenting, &options));
            let report = match result {
                Ok((_, wrong, runs)) if wrong.is_empty() => (None, runs),
                Ok((input, wrong, runs)) => {
                    (Some(format!("seed {seed}, `{input}`: {wrong:#?}")), runs)
                }
                Err(_) => (Some(format!("seed {seed} panicked")), 0),
            };
            if send.send(report).is_err() {
                return;
            }
        }
    });
    let mut wrong = Vec::new();
    let mut runs = 0;
    for seed in 0..cases {
        match receive.recv_timeout(Duration::from_secs(5)) {
            Ok((report, ran)) => {
                wrong.extend(report);
                runs += ran;
            }
            Err(_) => panic!("seed {seed} did not return within 5 s"),
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {cases} cases: {:#?}",
        wrong.len(),
        &wrong[..wrong.len().min(5)]
    );
    assert!(runs as u64 >= cases, "{runs} hooks ran in {cases} cases");
}

/// Random writes of every kind, from each `run` on the nodes the hook has seen and made, and
/// random markup: every parse returns, every tree is a tree, and every script element the
/// parser inserted gets one call and no other node gets one.
#[test]
fn random_writes_and_markup_keep_a_tree_and_one_call_each() {
    random_cases(20_000, true, scripted());
}

/// The same at a depth limit of 12, which the writes' new parents make the parser check on
/// the tree and which stops many parses inside a script.
#[test]
fn random_writes_and_markup_at_a_low_depth_limit_keep_a_tree_and_one_call_each() {
    random_cases(10_000, true, deep(12));
}

/// Random writes that give no node a new parent (removal, adoption, attributes, data,
/// normalize), with the switch off, and random markup at a depth limit of 10: the same holds.
#[test]
fn random_writes_that_give_no_new_parent_at_a_low_depth_limit_keep_a_tree() {
    random_cases(10_000, false, deep(10));
}
