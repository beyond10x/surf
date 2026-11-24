//! Adversarial cases against `story:html-script-hook`: the hook the tree builder calls at the
//! end of a script element, and the public write surface of `Tree` used while the parser runs.
//! Each test names what it holds the unit to: the design note's Q1, Q2 and Q4
//! (`docs/design/scripts.md`), the limits of the parse (`src/limits.rs`) and the module
//! documentation of `src/tree.rs`. Every tree a case makes is checked as a tree by [`faults`].

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use surf_html::limits::{Limit, Limits};
use surf_html::{
    Namespace, NodeData, NodeId, NotRun, ParseOptions, ScriptHook, Tree, WriteError,
    parse_document, parse_document_with_hook,
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

fn recorded(input: &str, options: &ParseOptions) -> (Tree, Vec<Call>) {
    let mut hook = Recorder::default();
    let tree = parse_document_with_hook(input, options, &mut hook);
    (tree, hook.calls)
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

fn attribute_count(tree: &Tree, node: NodeId) -> usize {
    match tree.data(node) {
        NodeData::Element { attributes, .. } => attributes.len(),
        _ => 0,
    }
}

fn contents_of(tree: &Tree, template: NodeId) -> NodeId {
    match tree.data(template) {
        NodeData::Element {
            template_contents: Some(contents),
            ..
        } => *contents,
        _ => panic!("not a template"),
    }
}

/// Every node reached from the root through children and template contents, each once (a
/// cycle does not make the walk loop).
fn connected(tree: &Tree) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut pending = vec![tree.root()];
    while let Some(node) = pending.pop() {
        if !seen.insert(node) {
            continue;
        }
        out.push(node);
        if let NodeData::Element {
            template_contents: Some(contents),
            ..
        } = tree.data(node)
        {
            pending.push(*contents);
        }
        pending.extend(tree.children(node).iter().rev());
    }
    out
}

fn by_name(tree: &Tree, local_name: &str) -> NodeId {
    connected(tree)
        .into_iter()
        .find(
            |&n| matches!(tree.data(n), NodeData::Element { local_name: l, .. } if l == local_name),
        )
        .unwrap_or_else(|| panic!("no {local_name}"))
}

/// Every HTML or SVG `script` element reached from the root.
fn script_elements(tree: &Tree) -> Vec<NodeId> {
    connected(tree)
        .into_iter()
        .filter(|&n| {
            matches!(tree.data(n), NodeData::Element {
                namespace: Namespace::Html | Namespace::Svg,
                local_name,
                ..
            } if local_name == "script")
        })
        .collect()
}

/// The most elements on a path down from the root, a template counted for the nodes of its
/// contents (`src/tree.rs`, "Depth for a write").
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
        if let NodeData::Element {
            template_contents: Some(contents),
            ..
        } = tree.data(node)
        {
            pending.push((*contents, depth));
        }
    }
    most
}

/// What keeps a tree from being a tree, among the nodes reached from the root and from `known`
/// through children, parents and template contents: a child whose parent is another node, a
/// child listed twice, a node listed by a parent it does not name, a document that is not the
/// root, a template's contents with a parent, and a cycle of parents and template hosts. The
/// walk is bounded by the arena, so the check itself never loops.
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
        if let NodeData::Element {
            template_contents: Some(contents),
            ..
        } = tree.data(node)
        {
            hosts.insert(*contents, node);
            if tree.parent(*contents).is_some() {
                found.push(format!("the contents {contents:?} have a parent"));
            }
            pending.push(*contents);
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

// Item 7 of the brief: the hook's calls. The note's reading: every script element the parser
// inserts gets one call, never two and never none; an SVG `script` gets `svg` however it ends.

/// `</svg>` pops the SVG `script` with the `svg` ("any other end tag" of 13.2.6.5): the
/// element was inserted by the parser and gets no call.
#[test]
fn an_svg_script_closed_by_the_svg_end_tag_gets_one_call() {
    let input = "<svg><script id=a>x = 1</svg><p>after";
    let (tree, calls) = recorded(input, &scripted());
    assert_a_tree(&tree, &[], input);
    assert_eq!(script_elements(&tree).len(), 1);
    assert_eq!(named(&tree, &calls), [("a".to_string(), Some(NotRun::Svg))]);
}

/// An SVG `script` still open at the end of the input gets no call.
#[test]
fn an_svg_script_open_at_the_end_of_the_input_gets_one_call() {
    let input = "<svg><script id=a>x = 1";
    let (tree, calls) = recorded(input, &scripted());
    assert_a_tree(&tree, &[], input);
    assert_eq!(script_elements(&tree).len(), 1);
    assert_eq!(named(&tree, &calls), [("a".to_string(), Some(NotRun::Svg))]);
}

/// The other ends of an inserted script element that make no call: an HTML end tag or a
/// breakout start tag in foreign content, the SVG `script` end tag while an SVG child of the
/// script is the current node, and a limit that stops the parse inside an HTML `script`.
#[test]
fn every_other_end_of_an_inserted_script_gets_one_call() {
    let base = parse_document("<script id=a>", &scripted()).stored_bytes();
    let tight = scripted_with(Limits {
        max_stored_bytes: base + 3,
        ..Limits::default()
    });
    let open = scripted();
    let cases: [(&str, &str, &ParseOptions, Option<NotRun>); 4] = [
        (
            "an HTML end tag",
            "<svg><script id=a>x</p>after",
            &open,
            Some(NotRun::Svg),
        ),
        (
            "a breakout start tag",
            "<svg><script id=a>x<div>after",
            &open,
            Some(NotRun::Svg),
        ),
        (
            "its end tag under an SVG child",
            "<svg><script id=a><g></script>after</svg>",
            &open,
            Some(NotRun::Svg),
        ),
        (
            "the stored-byte limit inside its text",
            "<script id=a>abcdefgh</script><p>after",
            &tight,
            None,
        ),
    ];
    let mut wrong = Vec::new();
    for (what, input, options, reason) in cases {
        let (tree, calls) = recorded(input, options);
        assert_a_tree(&tree, &[], input);
        let scripts = script_elements(&tree);
        let got: Vec<Option<NotRun>> = calls.iter().map(|c| c.1).collect();
        let one_call = scripts.len() == 1 && calls.len() == 1 && calls[0].0 == scripts[0];
        let right_word = reason.is_none() || got == [reason];
        if !one_call || !right_word {
            wrong.push(format!(
                "{what}: `{input}`: {} script element(s), calls {got:?}, stopped by {:?}",
                scripts.len(),
                tree.stopped_by()
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The adoption agency algorithm checks that the node it inserts is not an ancestor of the
/// place (its step 4.23), but it looks a template's contents' host up on the stack of open
/// elements only. A script moves the common ancestor into the contents of a template that is
/// not open and hangs that template under the furthest block, which it took out of the tree;
/// `</b>` then inserts the furthest block into the common ancestor: a cycle through the
/// template's contents. The tree is no longer a tree.
#[test]
fn the_adoption_agency_after_writes_makes_no_cycle() {
    let input = "<body><div id=ca><b id=fmt><p id=fb><script id=s>x</script></b>after";
    let mut hook = Recorder::with(|tree, script| {
        let furthest = tree.parent(script).unwrap();
        let formatting = tree.parent(furthest).unwrap();
        let common = tree.parent(formatting).unwrap();
        tree.remove_node(furthest).unwrap();
        let template = tree.create_element_ns(Namespace::Html, "template").unwrap();
        tree.append_child(furthest, template).unwrap();
        let contents = contents_of(tree, template);
        tree.append_child(contents, common).unwrap();
        assert_a_tree(tree, &[script, furthest, template], "after the writes");
    });
    let tree = parse_document_with_hook(input, &scripted(), &mut hook);
    let script = hook.calls[0].0;
    assert_a_tree(&tree, &[script], input);
}

/// The same markup and writes, then `</div>` and a second script, connected, that appends a
/// text node to the furthest block the first one kept (a script keeps a node in a variable):
/// the write's walk up the parents and template hosts (`insert_before`, "ensure pre-insertion
/// validity") goes round the cycle and never ends, so the parse never returns.
#[test]
fn a_later_script_write_returns_after_the_adoption_agency() {
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        let input = "<body><div id=ca><b id=fmt><p id=fb><script id=s>x</script></b></div>\
                     <script id=s2>y</script>after";
        let kept: Rc<RefCell<Option<NodeId>>> = Rc::default();
        let kept_in_hook = kept.clone();
        let mut hook = Recorder::with(move |tree, script| {
            if id_of(tree, script) == "s" {
                let furthest = tree.parent(script).unwrap();
                let formatting = tree.parent(furthest).unwrap();
                let common = tree.parent(formatting).unwrap();
                tree.remove_node(furthest).unwrap();
                let template = tree.create_element_ns(Namespace::Html, "template").unwrap();
                tree.append_child(furthest, template).unwrap();
                let contents = contents_of(tree, template);
                tree.append_child(contents, common).unwrap();
                *kept_in_hook.borrow_mut() = Some(furthest);
            } else if let Some(furthest) = *kept_in_hook.borrow() {
                let text = tree.create_text_node("written").unwrap();
                let _ = tree.append_child(furthest, text);
            }
        });
        let tree = parse_document_with_hook(input, &scripted(), &mut hook);
        let _ = done.send(named(&tree, &hook.calls));
    });
    let returned = finished.recv_timeout(Duration::from_secs(2));
    let calls = returned.expect(
        "the parse did not return within 2 s: the second script's append_child walks a cycle",
    );
    assert!(calls.contains(&("s2".to_string(), None)), "{calls:?}");
}

// Item 9 of the brief: the limits, reached by writes alone, by markup then writes, and by
// writes then markup.

/// The attribute limit, writes then markup. The `html` and `body` merge counts the names of
/// the element's attributes, not its attributes: a script that gives `body` two attributes of
/// one local name in two namespaces leaves room for one more name, and `<body c>` takes the
/// element past the limit.
#[test]
fn writes_then_a_body_merge_stay_inside_the_attribute_limit() {
    let limits = Limits {
        max_attributes: 2,
        ..Limits::default()
    };
    let input = "<body><script id=s>x</script><body c=3>after";
    let mut hook = Recorder::with(|tree, script| {
        let body = tree.parent(script).unwrap();
        tree.set_attribute(body, "x", "1").unwrap();
        tree.set_attribute_ns(body, Some(Namespace::XLink), "xlink:x", "2")
            .unwrap();
        let before = tree.clone();
        for _ in 0..1000 {
            assert_eq!(
                tree.set_attribute(body, "y", "3"),
                Err(WriteError::Limit(Limit::Attributes))
            );
        }
        assert_eq!(*tree, before);
    });
    let tree = parse_document_with_hook(input, &scripted_with(limits), &mut hook);
    assert_a_tree(&tree, &[], input);
    let body = by_name(&tree, "body");
    assert!(
        attribute_count(&tree, body) <= limits.max_attributes,
        "body holds {} attributes, the limit is {}: {:?}",
        attribute_count(&tree, body),
        limits.max_attributes,
        tree.data(body)
    );
}

/// The depth limit, writes then markup. A script moves an open element under a chain it made,
/// within the limit; the parser bounds its stack of open elements, not the depth of the
/// element it inserts into, so the markup after the script nests elements past the limit.
#[test]
fn writes_then_markup_stay_inside_the_depth_limit() {
    let limits = Limits {
        max_depth: 8,
        ..Limits::default()
    };
    let input = "<body><div id=d><script id=s>x</script><b><b><b><b><b><b><b><b>deep";
    let mut hook = Recorder::with(|tree, script| {
        let div = tree.parent(script).unwrap();
        let mut at = tree.parent(div).unwrap();
        for _ in 0..4 {
            let chain = tree.create_element_ns(Namespace::Html, "section").unwrap();
            tree.append_child(at, chain).unwrap();
            at = chain;
        }
        tree.append_child(at, div).unwrap();
        assert!(max_element_depth(tree) <= 8);
    });
    let tree = parse_document_with_hook(input, &scripted_with(limits), &mut hook);
    assert_a_tree(&tree, &[], input);
    assert!(
        max_element_depth(&tree) <= limits.max_depth,
        "an element {} deep, the limit is {}",
        max_element_depth(&tree),
        limits.max_depth
    );
}
