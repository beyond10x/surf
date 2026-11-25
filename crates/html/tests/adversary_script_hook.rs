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

// The cases below held: each is a case of the brief that the tip passes, or one that catches a
// change of the implementation the unit's own tests do not notice.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

/// A hook that writes nothing and counts its calls.
#[derive(Default)]
struct Counter {
    calls: usize,
}

impl ScriptHook for Counter {
    fn run(&mut self, _: &mut Tree, _: NodeId) {
        self.calls += 1;
    }

    fn skip(&mut self, _: &Tree, _: NodeId, _: NotRun) {
        self.calls += 1;
    }
}

fn by_id(tree: &Tree, id: &str) -> NodeId {
    connected(tree)
        .into_iter()
        .find(|&n| attribute(tree, n, "id") == Some(id))
        .unwrap_or_else(|| panic!("no element with the id {id}"))
}

/// An input and the calls it gives: the `id` of each element, and `None` for `run` or the
/// reason of `skip`.
type MarkupCase = (&'static str, &'static [(&'static str, Option<NotRun>)]);

/// Markup the table of Q1 does not spell out, and the call each gives. Each script element in
/// the tree gets exactly the calls listed.
#[test]
fn markup_q1_does_not_spell_out_gets_its_calls() {
    use NotRun::*;
    let cases: &[MarkupCase] = &[
        (
            "<template><template><script id=a>x</script></template></template>",
            &[("a", Some(Disconnected))],
        ),
        (
            "<select><script id=a>x</script><option>o</select>",
            &[("a", None)],
        ),
        (
            "<table><script id=a>x</script><tr><td>c</table>",
            &[("a", None)],
        ),
        ("<table><tr><script id=a>x</script><td>c", &[("a", None)]),
        ("<frameset><script id=a>x</script></frameset>", &[]),
        ("<head><noscript><script id=a>x</script></noscript>", &[]),
        ("<body><noscript><script id=a>x</script></noscript>", &[]),
        ("<plaintext><script id=a>x</script>", &[]),
        ("<title><script id=a>x</script></title>", &[]),
        ("<textarea><script id=a>x</script></textarea>", &[]),
        ("<title>x <script id=a>", &[]),
        ("<style>x <script id=a>", &[]),
        ("<body><xmp>x <script id=a>", &[]),
        ("<script id=a>x</div>more text", &[("a", Some(Eof))]),
        ("<script id=a>x</script", &[("a", Some(Eof))]),
        (
            "<script id=a><!--<script></script>--></script><script id=b>y</script>",
            &[("a", None), ("b", None)],
        ),
        ("<math><script id=a>x</script></math>", &[]),
        (
            "<math><mi><script id=a>x</script></mi></math>",
            &[("a", None)],
        ),
        (
            "<svg><foreignObject><script id=a>x</script></foreignObject></svg>",
            &[("a", None)],
        ),
        (
            "<svg><desc><script id=a>x</script></desc></svg>",
            &[("a", None)],
        ),
        ("<svg><script id=a /></svg>", &[("a", Some(Svg))]),
        (
            "<svg><script id=a>x</script><script id=b>y</script></svg>",
            &[("a", Some(Svg)), ("b", Some(Svg))],
        ),
        (
            "<svg><script id=a><script id=b /></script></svg>",
            &[("b", Some(Svg)), ("a", Some(Svg))],
        ),
        (
            "<template><svg><script id=a>x</script></svg></template>",
            &[("a", Some(Svg))],
        ),
        (
            "<script id=a type=text/plain type=text/javascript>x</script>",
            &[("a", Some(Type))],
        ),
        (
            "<SCRIPT id=a TYPE=Module>x</SCRIPT>",
            &[("a", Some(Module))],
        ),
        (
            "<script id=a type=\"  tExt/JavaScript \">x</script>",
            &[("a", None)],
        ),
        (
            "<script id=a type=\"text/javascript\0\">x</script>",
            &[("a", Some(Type))],
        ),
        (
            "<script id=a type=\"text/javascr\u{130}pt\">x</script>",
            &[("a", Some(Type))],
        ),
        (
            "<script id=a type=\"text/javascript\u{A0}\">x</script>",
            &[("a", Some(Type))],
        ),
        ("<script id=a type=\" \">x</script>", &[("a", Some(Type))]),
        (
            "<script id=a language=\" javascript\">x</script>",
            &[("a", Some(Type))],
        ),
        ("<script id=a src>x</script>", &[("a", Some(Src))]),
        ("<script id=a event=onload>x</script>", &[("a", None)]),
        (
            "<script id=a nomodule type=text/plain>x</script>",
            &[("a", Some(Type))],
        ),
        (
            "<script id=a>same</script><script id=b>same</script>",
            &[("a", None), ("b", None)],
        ),
        ("<head></head><script id=a>x</script><body>", &[("a", None)]),
        ("<body></body><script id=a>x</script>", &[("a", None)]),
        ("</html><script id=a>x</script>", &[("a", None)]),
    ];
    let mut wrong = Vec::new();
    for &(input, expected) in cases {
        let (tree, calls) = recorded(input, &scripted());
        assert_a_tree(&tree, &[], input);
        let got = named(&tree, &calls);
        let expected: Vec<(String, Option<NotRun>)> =
            expected.iter().map(|&(i, r)| (i.to_string(), r)).collect();
        let called: HashSet<NodeId> = calls.iter().map(|c| c.0).collect();
        let scripts: HashSet<NodeId> = script_elements(&tree).into_iter().collect();
        if got != expected || calls.len() != called.len() || called != scripts {
            wrong.push(format!("`{input}`: calls {got:?}, expected {expected:?}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
    // The script data escaped states keep the inner markup as the script's text.
    let (tree, _) = recorded(
        "<script id=a><!--<script></script>--></script>",
        &scripted(),
    );
    let a = by_id(&tree, "a");
    assert_eq!(
        tree.data(tree.children(a)[0]),
        &NodeData::Text("<!--<script></script>-->".to_string())
    );
}

/// 2,000 scripts in one document: 2,000 `run` calls, one for each element, in document order.
#[test]
fn two_thousand_scripts_get_one_call_each_in_order() {
    let input: String = (0..2000)
        .map(|i| format!("<script id=s{i}>x</script>"))
        .collect();
    let (tree, calls) = recorded(&input, &scripted());
    assert_a_tree(&tree, &[], "2,000 scripts");
    let expected: Vec<(String, Option<NotRun>)> =
        (0..2000).map(|i| (format!("s{i}"), None)).collect();
    assert_eq!(named(&tree, &calls), expected);
    let distinct: HashSet<NodeId> = calls.iter().map(|c| c.0).collect();
    assert_eq!(distinct.len(), 2000);
}

/// Q2: a removed ancestor keeps receiving the markup, so a later script in it is not connected.
/// A script that ran and was moved gets no second call.
#[test]
fn a_script_in_a_removed_ancestor_is_disconnected() {
    let input = "<body><div id=d><script id=a>x</script><script id=b>y</script></div>\
                 <script id=c>z</script>";
    let mut hook = Recorder::with(|tree, script| {
        if id_of(tree, script) == "a" {
            let div = tree.parent(script).unwrap();
            tree.remove_node(div).unwrap();
            let body = by_name(tree, "body");
            tree.append_child(body, script).unwrap();
        }
    });
    let tree = parse_document_with_hook(input, &scripted(), &mut hook);
    let known: Vec<NodeId> = hook.calls.iter().map(|c| c.0).collect();
    assert_a_tree(&tree, &known, input);
    assert_eq!(
        named(&tree, &hook.calls),
        [
            ("a".to_string(), None),
            ("b".to_string(), Some(NotRun::Disconnected)),
            ("c".to_string(), None),
        ]
    );
}

// Item 6 of the brief: without a hook nothing changes, over every vendored input.

/// The `#data` of every tree-construction case in the vendored `.dat` files.
fn vendored_inputs() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../html-conformance/data/wpt/html/syntax/parsing/resources");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "dat"))
        .collect();
    files.sort();
    let mut inputs = Vec::new();
    for file in files {
        let bytes = std::fs::read(&file).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        let mut data: Option<Vec<&str>> = None;
        for line in text.split('\n') {
            if line == "#data" {
                if let Some(lines) = data.take() {
                    inputs.push(lines.join("\n"));
                }
                data = Some(Vec::new());
            } else if line.len() > 1 && line.starts_with('#') {
                if let Some(lines) = data.take() {
                    inputs.push(lines.join("\n"));
                }
            } else if let Some(lines) = data.as_mut() {
                lines.push(line);
            }
        }
        if let Some(lines) = data.take() {
            inputs.push(lines.join("\n"));
        }
    }
    inputs
}

/// Each vendored input parsed with scripting enabled gives the same tree with a hook that
/// writes nothing as without a hook.
#[test]
fn a_hook_that_writes_nothing_leaves_every_vendored_tree_as_it_was() {
    let inputs = vendored_inputs();
    let mut calls = 0;
    let mut differ = Vec::new();
    for input in &inputs {
        let without = parse_document(input, &scripted());
        let mut hook = Counter::default();
        let with = parse_document_with_hook(input, &scripted(), &mut hook);
        calls += hook.calls;
        if with != without {
            differ.push(input.clone());
        }
    }
    eprintln!(
        "vendored inputs compared: {}, hook calls: {calls}, trees that differ: {}",
        inputs.len(),
        differ.len()
    );
    assert!(inputs.len() > 1000, "the data was read");
    assert!(differ.is_empty(), "{differ:#?}");
}

// Item 8 of the brief: writes while the parser runs, on every kind of node, then more markup.

const MATRIX_PREFIX: &str = "<!DOCTYPE html><html><head><title>t</title></head><body>\
    <div id=d><b id=b><i id=i><template id=t><span>c</span></template><script id=s>x</script>";

const MATRIX_SUFFIXES: [&str; 5] = [
    "</div>x<p>y",
    "<table>foster<tr><td>a</td></tr></table>z",
    "<body class=c id=j>w",
    "<p>1</b>2</i>3<p>4",
    "</html>text",
];

#[derive(Clone, Copy, Debug)]
enum Op {
    Remove,
    UnderOwnDescendant,
    IntoTemplateContents,
    ReplaceText,
    ThousandChildren,
}

const MATRIX_TARGETS: [&str; 11] = [
    "the script",
    "its parent, the node of the next insertion",
    "html",
    "head",
    "body",
    "the document",
    "the div on the stack",
    "the formatting b on the stack",
    "the template",
    "the template's contents",
    "the script's text",
];

/// The node a target of [`MATRIX_TARGETS`] names, when the hook runs for `script`.
fn pick(tree: &Tree, script: NodeId, which: &str) -> NodeId {
    match which {
        "the script" => script,
        "its parent, the node of the next insertion" => tree.parent(script).unwrap(),
        "html" => by_name(tree, "html"),
        "head" => by_name(tree, "head"),
        "body" => by_name(tree, "body"),
        "the document" => tree.root(),
        "the div on the stack" => by_id(tree, "d"),
        "the formatting b on the stack" => by_id(tree, "b"),
        "the template" => by_id(tree, "t"),
        "the template's contents" => contents_of(tree, by_id(tree, "t")),
        "the script's text" => tree.children(script)[0],
        _ => unreachable!("a target of the list"),
    }
}

/// One write of `op` on `target`: every result is allowed but a move under the target's own
/// descendant, which must be refused.
fn write(tree: &mut Tree, target: NodeId, op: Op, found: &mut Vec<String>) {
    match op {
        Op::Remove => {
            let _ = tree.remove_node(target);
        }
        Op::UnderOwnDescendant => {
            let below = tree
                .descendants(target)
                .into_iter()
                .skip(1)
                .find(|&n| is_element(tree, n));
            let below = match below {
                Some(node) => node,
                None => match tree.create_element_ns(Namespace::Html, "em") {
                    Ok(em) => {
                        if tree.append_child(target, em).is_ok() {
                            em
                        } else {
                            target
                        }
                    }
                    Err(_) => target,
                },
            };
            let results = [
                tree.append_child(below, target),
                tree.insert_before(below, target, None),
                tree.move_before(below, target, None),
            ];
            for result in results {
                if result.is_ok() {
                    found.push("a move under its own descendant was made".to_string());
                }
            }
        }
        Op::IntoTemplateContents => {
            let template = by_id(tree, "t");
            let contents = contents_of(tree, template);
            let _ = tree.append_child(contents, target);
        }
        Op::ReplaceText => {
            let _ = tree.set_text_content(target, "new");
        }
        Op::ThousandChildren => {
            for _ in 0..1000 {
                let Ok(em) = tree.create_element_ns(Namespace::Html, "em") else {
                    break;
                };
                let _ = tree.append_child(target, em);
            }
        }
    }
}

/// Each write on each node the brief names, then markup that closes, foster-parents, merges
/// `body` attributes, runs the adoption agency algorithm or ends the document: no panic, and
/// the tree stays a tree after the hook and at the end.
#[test]
fn writes_on_every_node_while_the_parser_runs_keep_a_tree() {
    let ops = [
        Op::Remove,
        Op::UnderOwnDescendant,
        Op::IntoTemplateContents,
        Op::ReplaceText,
        Op::ThousandChildren,
    ];
    let mut wrong = Vec::new();
    let mut cases = 0;
    for what in MATRIX_TARGETS {
        for op in ops {
            for suffix in MATRIX_SUFFIXES {
                cases += 1;
                let label = format!("{what} / {op:?} / `{suffix}`");
                let found: Rc<RefCell<Vec<String>>> = Rc::default();
                let known: Rc<RefCell<Vec<NodeId>>> = Rc::default();
                let (found_in_hook, known_in_hook) = (found.clone(), known.clone());
                let input = format!("{MATRIX_PREFIX}{suffix}");
                let outcome = catch_unwind(AssertUnwindSafe(|| {
                    let mut hook = Recorder::with(move |tree, script| {
                        if id_of(tree, script) != "s" {
                            return;
                        }
                        let node = pick(tree, script, what);
                        known_in_hook.borrow_mut().extend([script, node]);
                        let mut here = Vec::new();
                        write(tree, node, op, &mut here);
                        here.extend(faults(tree, &[script, node]));
                        found_in_hook
                            .borrow_mut()
                            .extend(here.into_iter().map(|f| format!("in the hook: {f}")));
                    });
                    let tree = parse_document_with_hook(&input, &scripted(), &mut hook);
                    let after = faults(&tree, &known.borrow());
                    found
                        .borrow_mut()
                        .extend(after.into_iter().map(|f| format!("at the end: {f}")));
                }));
                if let Err(panic) = outcome {
                    let message = panic
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_default();
                    wrong.push(format!("{label}: panicked: {message}"));
                }
                for fault in found.borrow().iter() {
                    wrong.push(format!("{label}: {fault}"));
                }
            }
        }
    }
    assert_eq!(cases, 11 * 5 * 5);
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The node and stored-byte limits, writes then markup: a script fills each with writes, a
/// write one past is refused 1,000 times with the limit's own error and changes nothing, and
/// the markup after the script stops the parse at that limit (a script can make the parser stop
/// early) without passing it.
#[test]
fn writes_then_markup_stay_inside_the_node_and_byte_limits() {
    let input = "<body><script id=s>x</script><p>after</p>";
    let base = parse_document("<body><script id=s>x</script>", &scripted());
    for limit in [Limit::Nodes, Limit::StoredBytes] {
        let limits = match limit {
            Limit::Nodes => Limits {
                max_nodes: base.node_count() + 50,
                ..Limits::default()
            },
            _ => Limits {
                max_stored_bytes: base.stored_bytes() + 500,
                ..Limits::default()
            },
        };
        let mut hook = Recorder::with(move |tree, script| {
            let body = tree.parent(script).unwrap();
            loop {
                match tree.create_text_node("0123456789") {
                    Ok(text) => tree.append_child(body, text).unwrap(),
                    Err(error) => {
                        assert_eq!(error, WriteError::Limit(limit));
                        break;
                    }
                }
            }
            let before = tree.clone();
            for _ in 0..1000 {
                assert_eq!(
                    tree.create_text_node("0123456789"),
                    Err(WriteError::Limit(limit))
                );
            }
            assert_eq!(*tree, before);
        });
        let tree = parse_document_with_hook(input, &scripted_with(limits), &mut hook);
        assert_a_tree(&tree, &[], input);
        assert_eq!(hook.calls.len(), 1);
        assert_eq!(tree.stopped_by(), Some(limit), "{limit:?}");
        assert!(tree.node_count() <= limits.max_nodes);
        assert!(tree.stored_bytes() <= limits.max_stored_bytes);
    }
}

/// Markup then writes: the markup reaches each limit exactly, and the script's first write
/// past it is refused with the limit's error and changes nothing; the parse ends whole.
#[test]
fn markup_then_writes_are_refused_at_each_limit() {
    let input = "<body a=1 b=2><div id=d><script id=s>x</script>";
    let base = parse_document(input, &scripted());
    let cases = [
        (
            Limits {
                max_nodes: base.node_count(),
                ..Limits::default()
            },
            Limit::Nodes,
        ),
        (
            Limits {
                max_stored_bytes: base.stored_bytes(),
                ..Limits::default()
            },
            Limit::StoredBytes,
        ),
        (
            Limits {
                max_attributes: 2,
                ..Limits::default()
            },
            Limit::Attributes,
        ),
        (
            Limits {
                max_depth: 4,
                ..Limits::default()
            },
            Limit::Depth,
        ),
    ];
    for (limits, limit) in cases {
        let mut hook = Recorder::with(move |tree, script| {
            let div = tree.parent(script).unwrap();
            let body = tree.parent(div).unwrap();
            let before = tree.clone();
            let refused = match limit {
                Limit::Nodes => tree.create_comment("").map(|_| ()),
                Limit::StoredBytes => tree.set_attribute(div, "id", "dd"),
                Limit::Attributes => tree.set_attribute(body, "c", "3"),
                _ => {
                    // `div` is three deep: a `span` in it is the fourth, the limit; an `em`
                    // in the `span` would be the fifth.
                    let span = tree.create_element_ns(Namespace::Html, "span").unwrap();
                    tree.append_child(div, span).unwrap();
                    let em = tree.create_element_ns(Namespace::Html, "em").unwrap();
                    let before = tree.clone();
                    let result = tree.append_child(span, em);
                    assert_eq!(*tree, before);
                    result
                }
            };
            assert_eq!(refused, Err(WriteError::Limit(limit)), "{limit:?}");
            if limit != Limit::Depth {
                assert_eq!(*tree, before, "{limit:?}");
            }
        });
        let tree = parse_document_with_hook(input, &scripted_with(limits), &mut hook);
        assert_eq!(hook.calls.len(), 1, "{limit:?}");
        assert_eq!(tree.stopped_by(), None, "{limit:?}");
        assert_a_tree(&tree, &[], input);
    }
}

/// Writes alone: a refused write leaves no count raised (1,000 refusals, then a write that
/// fits is made), and what a removal or a shorter value gives back, as `src/tree.rs` says: a
/// removed node keeps its count and its strings; data made shorter and a removed attribute give
/// their bytes back; a removed attribute gives its place back.
#[test]
fn refused_writes_leave_no_count_raised() {
    let input = "<p id=p>text</p>";
    let base = parse_document(input, &scripted());
    // Nodes: a template is two nodes, one is left.
    let mut tree = parse_document(
        input,
        &scripted_with(Limits {
            max_nodes: base.node_count() + 1,
            ..Limits::default()
        }),
    );
    let before = tree.clone();
    for _ in 0..1000 {
        assert_eq!(
            tree.create_element_ns(Namespace::Html, "template"),
            Err(WriteError::Limit(Limit::Nodes))
        );
    }
    assert_eq!(tree, before);
    let span = tree.create_element_ns(Namespace::Html, "span").unwrap();
    let p = by_id(&tree, "p");
    tree.append_child(p, span).unwrap();
    tree.remove_node(span).unwrap();
    assert_eq!(
        tree.create_comment(""),
        Err(WriteError::Limit(Limit::Nodes)),
        "a removed node keeps its count"
    );
    assert_a_tree(&tree, &[span], "nodes");
    // Stored bytes: 10 are left.
    let mut tree = parse_document(
        input,
        &scripted_with(Limits {
            max_stored_bytes: base.stored_bytes() + 10,
            ..Limits::default()
        }),
    );
    let before = tree.clone();
    for _ in 0..1000 {
        assert_eq!(
            tree.create_text_node("0123456789a"),
            Err(WriteError::Limit(Limit::StoredBytes))
        );
    }
    assert_eq!(tree, before);
    let text = tree.create_text_node("0123456789").unwrap();
    tree.set_data(text, "").unwrap();
    let other = tree.create_text_node("0123456789").unwrap();
    tree.remove_node(other).unwrap();
    assert_eq!(
        tree.create_text_node("x"),
        Err(WriteError::Limit(Limit::StoredBytes))
    );
    assert_a_tree(&tree, &[text, other], "stored bytes");
    // Attributes: `p` may hold two.
    let mut tree = parse_document(
        input,
        &scripted_with(Limits {
            max_attributes: 2,
            ..Limits::default()
        }),
    );
    let p = by_id(&tree, "p");
    tree.set_attribute(p, "a", "1").unwrap();
    let before = tree.clone();
    for _ in 0..1000 {
        assert_eq!(
            tree.toggle_attribute(p, "b", None),
            Err(WriteError::Limit(Limit::Attributes))
        );
    }
    assert_eq!(tree, before);
    tree.set_attribute(p, "a", "2").unwrap();
    tree.remove_attribute(p, "a").unwrap();
    tree.set_attribute(p, "b", "1").unwrap();
    assert_eq!(attribute_count(&tree, p), 2);
    // Depth: `p` is three deep, the limit.
    let mut tree = parse_document(
        input,
        &scripted_with(Limits {
            max_depth: 3,
            ..Limits::default()
        }),
    );
    let p = by_id(&tree, "p");
    let em = tree.create_element_ns(Namespace::Html, "em").unwrap();
    let before = tree.clone();
    for _ in 0..1000 {
        assert_eq!(
            tree.append_child(p, em),
            Err(WriteError::Limit(Limit::Depth))
        );
    }
    assert_eq!(tree, before);
    let body = tree.parent(p).unwrap();
    tree.append_child(body, em).unwrap();
    assert_a_tree(&tree, &[], "depth");
}

/// A write of character data at the offsets a case gives.
type DataWrite = fn(&mut Tree, NodeId) -> Result<(), WriteError>;

/// Offsets a caller controls: past the end, `usize::MAX`, between the two units of a surrogate
/// pair, and counts past the end. Each refused write leaves the tree as it was.
#[test]
fn offsets_past_the_end_and_inside_a_pair_are_refused() {
    let mut tree = parse_document("<p id=p>x</p>", &scripted());
    let p = by_id(&tree, "p");
    let text = tree.create_text_node("a\u{1F600}b").unwrap();
    tree.append_child(p, text).unwrap();
    let refused: [(&str, DataWrite); 8] = [
        ("insert at usize::MAX", |t, n| {
            t.insert_data(n, usize::MAX, "x")
        }),
        ("insert inside the pair", |t, n| t.insert_data(n, 2, "x")),
        ("insert one past the end", |t, n| t.insert_data(n, 5, "x")),
        ("delete from inside the pair", |t, n| t.delete_data(n, 2, 1)),
        ("delete to inside the pair", |t, n| t.delete_data(n, 1, 1)),
        ("replace at usize::MAX", |t, n| {
            t.replace_data(n, usize::MAX, usize::MAX, "")
        }),
        ("split at usize::MAX", |t, n| {
            t.split_text(n, usize::MAX).map(|_| ())
        }),
        ("split inside the pair", |t, n| {
            t.split_text(n, 2).map(|_| ())
        }),
    ];
    for (what, write) in refused {
        let before = tree.clone();
        assert_eq!(write(&mut tree, text), Err(WriteError::IndexSize), "{what}");
        assert_eq!(tree, before, "{what}");
    }
    tree.delete_data(text, 4, usize::MAX).unwrap();
    tree.replace_data(text, 1, usize::MAX, "c").unwrap();
    assert_eq!(*tree.data(text), NodeData::Text("ac".to_string()));
    let tail = tree.split_text(text, 2).unwrap();
    assert_eq!(*tree.data(tail), NodeData::Text(String::new()));
    assert_a_tree(&tree, &[], "offsets");
}

// Item 1 of the brief: cases that catch a change of the implementation the unit's own tests
// do not notice (each named in the report with its diff).

/// A parse with a hook whose input is cut keeps the limit that cut it, as `parse_document`
/// does (`lib.rs`, `parse_document_with_hook`'s `stop_at`).
#[test]
fn a_cut_input_is_named_with_a_hook_too() {
    let options = scripted_with(Limits {
        max_input_bytes: 20,
        ..Limits::default()
    });
    let input = "<script>a</script><p>after the cut";
    let tree = parse_document_with_hook(input, &options, &mut Recorder::default());
    assert_eq!(tree, parse_document(input, &options));
    assert_eq!(tree.stopped_by(), Some(Limit::InputBytes));
}

/// The depth of a template's contents counts the template and its ancestors: an element
/// appended to the contents of a template three deep is refused at a depth limit of 3.
#[test]
fn an_element_in_template_contents_counts_the_template_for_depth() {
    let mut tree = parse_document(
        "<template id=t></template>",
        &scripted_with(Limits {
            max_depth: 3,
            ..Limits::default()
        }),
    );
    assert_eq!(tree.stopped_by(), None);
    let contents = contents_of(&tree, by_id(&tree, "t"));
    let span = tree.create_element_ns(Namespace::Html, "span").unwrap();
    let before = tree.clone();
    assert_eq!(
        tree.append_child(contents, span),
        Err(WriteError::Limit(Limit::Depth))
    );
    assert_eq!(tree, before);
}

/// A shallow clone of a template is two nodes, the copy and its contents.
#[test]
fn a_template_clone_is_two_nodes_at_the_node_limit() {
    let input = "<template id=t></template>";
    let base = parse_document(input, &scripted());
    let mut tree = parse_document(
        input,
        &scripted_with(Limits {
            max_nodes: base.node_count() + 1,
            ..Limits::default()
        }),
    );
    let template = by_id(&tree, "t");
    let before = tree.clone();
    assert_eq!(
        tree.clone_node(template, false),
        Err(WriteError::Limit(Limit::Nodes))
    );
    assert_eq!(tree, before);
    assert!(tree.node_count() <= tree.limits().max_nodes);
}

/// `insertBefore` with the node as its own reference and `replaceChild` with the node's next
/// sibling keep the order the DOM gives.
#[test]
fn a_node_as_its_own_reference_keeps_its_place() {
    let mut tree = parse_document("<p id=p><i></i><b></b><u></u></p>", &scripted());
    let p = by_id(&tree, "p");
    let [i, b, _] = [0, 1, 2].map(|k| tree.children(p)[k]);
    tree.insert_before(p, b, Some(b)).unwrap();
    let order = |t: &Tree| -> Vec<String> {
        t.children(p)
            .iter()
            .map(|&n| match t.data(n) {
                NodeData::Element { local_name, .. } => local_name.clone(),
                _ => String::new(),
            })
            .collect()
    };
    assert_eq!(order(&tree), ["i", "b", "u"]);
    tree.replace_child(p, b, i).unwrap();
    assert_eq!(order(&tree), ["b", "u"]);
    assert_a_tree(&tree, &[i], "order");
}

/// `moveBefore` moves a comment, and `setAttribute` finds a prefixed attribute by its
/// qualified name.
#[test]
fn a_comment_moves_and_a_prefixed_attribute_is_found_by_its_qualified_name() {
    let mut tree = parse_document(
        "<p id=p><!--c--></p><div id=d></div><svg><a id=a xlink:href=x></a></svg>",
        &scripted(),
    );
    let p = by_id(&tree, "p");
    let d = by_id(&tree, "d");
    let comment = tree.children(p)[0];
    tree.move_before(d, comment, None).unwrap();
    assert_eq!(tree.children(d), [comment]);
    let a = by_id(&tree, "a");
    tree.set_attribute(a, "xlink:href", "y").unwrap();
    assert_eq!(attribute_count(&tree, a), 2);
    let NodeData::Element { attributes, .. } = tree.data(a) else {
        panic!("an element");
    };
    assert!(
        attributes
            .iter()
            .any(|at| at.local_name == "href" && at.value == "y")
    );
    assert_a_tree(&tree, &[], "moves");
}

/// The document takes a doctype only before its element.
#[test]
fn the_document_takes_a_doctype_only_before_its_element() {
    let mut tree = parse_document("<!DOCTYPE html><p>", &scripted());
    let root = tree.root();
    let doctype = tree.children(root)[0];
    let html = tree.children(root)[1];
    tree.remove_node(doctype).unwrap();
    let before = tree.clone();
    assert_eq!(
        tree.append_child(root, doctype),
        Err(WriteError::HierarchyRequest)
    );
    assert_eq!(tree, before);
    tree.insert_before(root, doctype, Some(html)).unwrap();
    assert_eq!(tree.children(root), [doctype, html]);
    assert_a_tree(&tree, &[], "doctype");
}

/// A node of another tree: an index past this tree's arena panics, as the write surface's
/// documentation says ("Panics: each when a NodeId it is given is not of this tree").
#[test]
fn a_node_of_another_tree_past_the_arena_panics_as_documented() {
    let big = parse_document(&"<p>x".repeat(50), &scripted());
    let foreign = *connected(&big).last().unwrap();
    let mut small = parse_document("<p>", &scripted());
    assert!(foreign_is_past(&small, &big, foreign));
    let body = by_name(&small, "body");
    let outcome = catch_unwind(AssertUnwindSafe(|| small.append_child(body, foreign)));
    assert!(outcome.is_err());
}

fn foreign_is_past(small: &Tree, big: &Tree, foreign: NodeId) -> bool {
    big.node_count() > small.node_count() && !connected(small).contains(&foreign)
}
