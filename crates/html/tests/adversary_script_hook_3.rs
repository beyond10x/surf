//! Adversarial cases of the third review of `story:html-script-hook`, against fix round 2:
//! the depth limit as a bound of the tree in every parse, kept in each node's record
//! (`Tree::depth`), with no switch and no cycle refusal in `Tree::insert`. Each test names what
//! it holds the unit to: the rulings of fix round 2, the crate documentation's "Limits" and the
//! documentation of `src/limits.rs`.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::mpsc;
use std::time::Duration;

use surf_html::limits::{Limit, Limits};
use surf_html::{
    Context, Namespace, NodeData, NodeId, NotRun, ParseOptions, ScriptHook, Tree, parse_document,
    parse_document_with_hook, parse_fragment,
};

/// One call of a hook: the element, and `None` for `run` or the reason of `skip`.
type Call = (NodeId, Option<NotRun>);

/// What a recording hook does in `run` after it recorded the call.
type Action = Box<dyn FnMut(&mut Tree, NodeId)>;

/// What a recording hook does in `skip` after it recorded the call.
type Look = Box<dyn FnMut(&Tree, NodeId)>;

/// A hook that records each call, runs an action in `run` and a look in `skip`.
#[derive(Default)]
struct Recorder {
    calls: Vec<Call>,
    on_run: Option<Action>,
    on_skip: Option<Look>,
}

impl Recorder {
    fn with(action: impl FnMut(&mut Tree, NodeId) + 'static) -> Recorder {
        Recorder {
            on_run: Some(Box::new(action)),
            ..Recorder::default()
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

    fn skip(&mut self, tree: &Tree, script: NodeId, reason: NotRun) {
        self.calls.push((script, Some(reason)));
        if let Some(look) = self.on_skip.as_mut() {
            look(tree, script);
        }
    }
}

fn deep(max_depth: usize) -> ParseOptions {
    ParseOptions {
        scripting_disabled: false,
        limits: Limits {
            max_depth,
            ..Limits::default()
        },
    }
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

fn is_element(tree: &Tree, node: NodeId) -> bool {
    matches!(tree.data(node), NodeData::Element { .. })
}

fn name(tree: &Tree, node: NodeId) -> &str {
    match tree.data(node) {
        NodeData::Element { local_name, .. } => local_name,
        _ => "",
    }
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

fn is_script_element(tree: &Tree, node: NodeId) -> bool {
    matches!(tree.data(node), NodeData::Element {
        namespace: Namespace::Html | Namespace::Svg,
        local_name,
        ..
    } if local_name == "script")
}

/// Every node reached from `starts` through children, template contents and parents, each
/// once.
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

fn by_id(tree: &Tree, id: &str) -> Option<NodeId> {
    connected(tree)
        .into_iter()
        .find(|&n| attribute(tree, n, "id") == Some(id))
}

/// The document as the root reaches it, in tree order: each node's level and data, a
/// template's contents after the template. Nodes out of the tree and the steps do not count,
/// as they do for `==` of two trees.
fn shape(tree: &Tree) -> Vec<(usize, NodeData)> {
    let mut out = Vec::new();
    let mut pending = vec![(tree.root(), 0)];
    while let Some((node, level)) = pending.pop() {
        out.push((level, tree.data(node).clone()));
        pending.extend(tree.children(node).iter().rev().map(|&c| (c, level + 1)));
        if let Some(contents) = contents_of(tree, node) {
            pending.push((contents, level + 1));
        }
    }
    out
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

/// What keeps a tree from being a tree, among the nodes reached from the root and from
/// `known`: a child whose parent is another node, a child listed twice, a node listed by a
/// parent it does not name, a document that is not the root, a template's contents with a
/// parent, and a cycle of parents and template hosts. Bounded by the arena.
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

// The cases from here to the next such line fail on the tip they were written against
// (`d69866b`).

/// The markup of the two cases below: `a` holds the script; after the script, markup 5
/// elements deep goes under `a`, then the page goes on in the `body`.
const UNDER_A_REMOVED_DIV: &str = "<body><div id=a><script id=s>x</script>\
    <p><span><em><i><u>y</u></i></em></span></p></div><p id=after>z</p>";

/// A node out of the tree keeps the depth it had in its last place (`tree.rs`, `Depth`), and
/// the parser checks that kept depth. The crate documentation, "Limits", says an element "is
/// not created under a parent 512 deep", depth being "the elements on its path from the
/// root", and Q2 of `docs/design/scripts.md` has later markup go into a removed element. Two
/// hooks leave the same document: the first removes `a`; the second moves `a` to the end of
/// a chain of six `div` it made out of the tree, then removes it, so `a` keeps a depth of 7
/// though nothing is above it. At a limit of 8 the first parse goes on to `p#after`; the
/// second stops at `Depth` at the `span`, 3 deep in its own tree, and the rest of the page,
/// which is within the limit, is lost. The same with the chain in the document.
#[test]
fn a_removed_element_is_not_checked_at_a_depth_it_no_longer_has() {
    const N: usize = 8;
    let removed = parse_document_with_hook(
        UNDER_A_REMOVED_DIV,
        &deep(N),
        &mut Recorder::with(|tree, script| {
            let a = tree.parent(script).unwrap();
            tree.remove_node(a).unwrap();
        }),
    );
    assert_eq!(removed.stopped_by(), None);
    assert!(by_id(&removed, "after").is_some());
    let mut wrong = Vec::new();
    for in_document in [false, true] {
        let mut hook = Recorder::with(move |tree, script| {
            let a = tree.parent(script).unwrap();
            let body = tree.parent(a).unwrap();
            let (first, last) = chain(tree, if in_document { N - 4 } else { N - 2 });
            if in_document {
                tree.append_child(body, first).unwrap();
            }
            tree.append_child(last, a).unwrap();
            tree.remove_node(a).unwrap();
            if in_document {
                tree.remove_node(first).unwrap();
            }
        });
        let tree = parse_document_with_hook(UNDER_A_REMOVED_DIV, &deep(N), &mut hook);
        assert_a_tree(&tree, &[], UNDER_A_REMOVED_DIV);
        if shape(&tree) != shape(&removed) || tree.stopped_by() != removed.stopped_by() {
            wrong.push(format!(
                "a moved to a chain {} and removed: stopped by {:?}, p#after {}",
                if in_document {
                    "in the document"
                } else {
                    "out of the tree"
                },
                tree.stopped_by(),
                if by_id(&tree, "after").is_some() {
                    "parsed"
                } else {
                    "lost"
                }
            ));
        }
    }
    assert!(wrong.is_empty(), "limit {N}: {wrong:#?}");
}

// The cases below held on the tip they were written against (`d69866b`).

/// Writes that move an open element away and back, into a template's contents and out, or
/// make a deep clone that stays out of the tree, change nothing in the end: the parse is the
/// parse of no hook, at a limit the markup after them reaches exactly (8) and at one below,
/// where both stop at `Depth`. Ruling 2 of fix round 2: "the tree of a hook whose writes
/// change nothing is the tree of no hook".
#[test]
fn writes_that_change_nothing_in_the_end_leave_the_parse_of_no_hook() {
    let input = "<body><div id=a><script id=s>x</script><p><span><em><i><u>y</u></i></em></span>\
                 </p></div><p id=after>z</p>";
    type Write = fn(&mut Tree, NodeId);
    fn away_and_back(tree: &mut Tree, a: NodeId) {
        let body = tree.parent(a).unwrap();
        let next = tree.children(body).iter().position(|&c| c == a).unwrap() + 1;
        let next = tree.children(body).get(next).copied();
        let html = tree.parent(body).unwrap();
        tree.remove_node(a).unwrap();
        let (first, last) = chain(tree, 2);
        tree.append_child(html, first).unwrap();
        tree.append_child(last, a).unwrap();
        tree.insert_before(body, a, next).unwrap();
        tree.remove_node(first).unwrap();
    }
    fn into_contents_and_out(tree: &mut Tree, a: NodeId) {
        let body = tree.parent(a).unwrap();
        let (first, last) = chain(tree, 2);
        let template = tree.create_element_ns(Namespace::Html, "template").unwrap();
        tree.append_child(last, template).unwrap();
        tree.append_child(body, first).unwrap();
        let contents = contents_of(tree, template).unwrap();
        tree.append_child(contents, a).unwrap();
        tree.insert_before(body, a, Some(first)).unwrap();
        tree.remove_node(first).unwrap();
    }
    fn a_deep_clone_out_of_the_tree(tree: &mut Tree, a: NodeId) {
        let (_, last) = chain(tree, 3);
        let copy = tree.clone_node(a, true).unwrap();
        tree.append_child(last, copy).unwrap();
    }
    fn up_one_and_back(tree: &mut Tree, a: NodeId) {
        let body = tree.parent(a).unwrap();
        let html = tree.parent(body).unwrap();
        tree.append_child(html, a).unwrap();
        tree.insert_before(body, a, tree.children(body).first().copied())
            .unwrap();
    }
    let writes: [(&str, Write); 4] = [
        ("away and back", away_and_back),
        ("into template contents and out", into_contents_and_out),
        ("a deep clone out of the tree", a_deep_clone_out_of_the_tree),
        ("up one level and back", up_one_and_back),
    ];
    let mut wrong = Vec::new();
    for n in [8, 7] {
        let without = parse_document(input, &deep(n));
        for (what, write) in writes {
            let mut hook = Recorder::with(move |tree, script| {
                let a = tree.parent(script).unwrap();
                write(tree, a);
            });
            let tree = parse_document_with_hook(input, &deep(n), &mut hook);
            assert_a_tree(&tree, &[], what);
            if shape(&tree) != shape(&without) || tree.stopped_by() != without.stopped_by() {
                wrong.push(format!(
                    "limit {n}, {what}: stopped by {:?} (no hook: {:?}), {} deep ({})",
                    tree.stopped_by(),
                    without.stopped_by(),
                    max_element_depth(&tree),
                    max_element_depth(&without)
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// A stop inside the adoption agency algorithm, at a limit of 10: the hook moves the
/// formatting element `b`, with the furthest block `p` under it, to the `html` element (`p`
/// 3 deep) and the `body` under a chain of two `div` (the `foreignObject` above `b` on the
/// stack 7 deep); three `span` go under `p`, 4 to 6 deep, the stack then full. `</b>` would
/// put `p` under the `foreignObject` (8), the new `b` under it (9) and the `span` 10 to 12
/// deep. The iteration makes nothing (one `b`, `p` still under it with its three `span`), the
/// parse names `Depth` and parses nothing after it, and the SVG `script` still open gets its
/// one call after the stop, with a tree that is a tree, 7 deep.
#[test]
fn a_stop_inside_the_adoption_agency_leaves_one_tree_to_the_last_call() {
    const N: usize = 10;
    let input = "<svg><script id=o><foreignObject><b id=f><p id=fb><script id=s>x</script>\
                 <span><span><span id=last>y</b><p id=after>z";
    let mut hook = Recorder::with(|tree, script| {
        let p = tree.parent(script).unwrap();
        let b = tree.parent(p).unwrap();
        let html = tree.root();
        let html = tree.children(html)[0];
        tree.append_child(html, b).unwrap();
        let body = tree.children(html)[1];
        let (first, last) = chain(tree, 2);
        tree.append_child(html, first).unwrap();
        tree.append_child(last, body).unwrap();
    });
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let keep = std::rc::Rc::clone(&seen);
    hook.on_skip = Some(Box::new(move |tree, script| {
        keep.borrow_mut().push((
            attribute(tree, script, "id").map(str::to_string),
            faults(tree, &[]),
            max_element_depth(tree),
        ));
    }));
    let tree = parse_document_with_hook(input, &deep(N), &mut hook);
    assert_eq!(tree.stopped_by(), Some(Limit::Depth));
    assert_a_tree(&tree, &[], input);
    assert!(max_element_depth(&tree) <= N);
    let b = by_id(&tree, "f").unwrap();
    let p = by_id(&tree, "fb").unwrap();
    assert_eq!(tree.parent(p), Some(b));
    let bs = connected(&tree)
        .into_iter()
        .filter(|&n| name(&tree, n) == "b")
        .count();
    assert_eq!(bs, 1, "the stopped iteration made no new b");
    let last = by_id(&tree, "last").expect("the third span was made");
    let span = tree.parent(last).unwrap();
    assert_eq!(tree.parent(tree.parent(span).unwrap()), Some(p));
    assert_eq!(by_id(&tree, "after"), None);
    assert_eq!(hook.calls.len(), 2);
    assert_eq!(hook.calls[1].1, Some(NotRun::Svg));
    assert_eq!(*seen.borrow(), [(Some("o".to_string()), vec![], 7)]);
}

/// The `p` the hook moved up to `html` is the furthest block of the `</b>`; the hook also
/// put the common ancestor `div#ca` 8 deep, under a chain of five `div`. The iteration would
/// put the clone of `i` 9 deep, the `p` 10 and the new `b` 11: past the limit of 10, so the
/// parse stops at `Depth` and the tree stays within the limit. The one clone counts.
#[test]
fn the_clones_of_an_iteration_count_toward_its_depth() {
    const N: usize = 10;
    let input = "<body><div><div id=ca><b><i><p id=fb><script id=s>x</script></b><p id=after>z";
    let mut hook = Recorder::with(|tree, script| {
        let p = tree.parent(script).unwrap();
        let ca = tree
            .parent(tree.parent(tree.parent(p).unwrap()).unwrap())
            .unwrap();
        let html = tree.children(tree.root())[0];
        let body = tree.children(html)[1];
        tree.append_child(html, p).unwrap();
        tree.remove_node(script).unwrap();
        let (first, last) = chain(tree, N - 5);
        tree.append_child(body, first).unwrap();
        tree.append_child(last, ca).unwrap();
    });
    let tree = parse_document_with_hook(input, &deep(N), &mut hook);
    assert_a_tree(&tree, &[], input);
    assert_eq!(tree.stopped_by(), Some(Limit::Depth));
    assert!(
        max_element_depth(&tree) <= N,
        "{}",
        max_element_depth(&tree)
    );
    assert_eq!(by_id(&tree, "after"), None);
}

/// The contents of a `template` under the furthest block count toward the depth its children
/// shift to: the hook moved `p` up to `html` and put under it a `template`, 3 deep, with
/// contents 7 `div` deep, 10 in all. The `</b>` would shift them 3 down; at a limit of 10 the
/// parse stops at `Depth`. (Markup alone cannot build the contents this deep under `p`: the
/// stack of open elements, 3 longer than `p`'s new path, reaches the limit first.)
#[test]
fn template_contents_under_the_furthest_block_count_toward_its_shift() {
    const N: usize = 10;
    let input = "<b><i><p id=fb><script id=s>x</script></b><p id=after>z";
    let mut hook = Recorder::with(|tree, script| {
        let p = tree.parent(script).unwrap();
        let html = tree.children(tree.root())[0];
        tree.append_child(html, p).unwrap();
        let template = tree.create_element_ns(Namespace::Html, "template").unwrap();
        let contents = contents_of(tree, template).unwrap();
        let (first, _) = chain(tree, N - 3);
        tree.append_child(contents, first).unwrap();
        tree.append_child(p, template).unwrap();
    });
    let tree = parse_document_with_hook(input, &deep(N), &mut hook);
    assert_a_tree(&tree, &[], input);
    assert_eq!(tree.stopped_by(), Some(Limit::Depth));
    assert!(
        max_element_depth(&tree) <= N,
        "{}",
        max_element_depth(&tree)
    );
    assert_eq!(by_id(&tree, "after"), None);
}

/// The walks of an iteration that shifts the furthest block's children down are steps
/// (`limits.rs`, `Steps`): the hook moved `p` up to `html`, so the `</b>` shifts its 1,002
/// nodes 3 down, walking them once for the deepest and once for the shift. Without the move
/// the shift is 0 and nothing is walked. The parse with the move takes at least 2,004 more
/// steps than the parse with a hook that writes nothing.
#[test]
fn the_walks_of_a_shift_down_are_steps() {
    const S: usize = 1_000;
    let input = format!(
        "<b><i><p id=fb><script id=s>x</script>{}</b>",
        "<br>".repeat(S)
    );
    let moved = parse_document_with_hook(
        &input,
        &deep(512),
        &mut Recorder::with(|tree, script| {
            let p = tree.parent(script).unwrap();
            let html = tree.children(tree.root())[0];
            tree.append_child(html, p).unwrap();
        }),
    );
    let still = parse_document_with_hook(&input, &deep(512), &mut Recorder::with(|_, _| {}));
    assert_eq!(moved.stopped_by(), None);
    assert_eq!(still.stopped_by(), None);
    let more = moved.steps() - still.steps();
    assert!(more >= 2 * (S + 2), "{more} more steps");
}

/// The `selectedcontent` copy cut at the limit: the copy is a chain of `i` from the top, as
/// deep as the limit lets it, under the `selectedcontent`, and the tree is a tree.
#[test]
fn a_selectedcontent_copy_cut_at_the_limit_is_a_prefix_of_the_option() {
    for n in [10, 16] {
        let input = format!(
            "<select><button>{}<selectedcontent id=sc></selectedcontent>{}</button><option>{}x",
            "<div>".repeat(n - 7),
            "</div>".repeat(n - 7),
            "<i>".repeat(n - 4)
        );
        let tree = parse_document(&input, &deep(n));
        assert_a_tree(&tree, &[], &input);
        assert_eq!(tree.stopped_by(), Some(Limit::Depth), "{input}");
        assert_eq!(max_element_depth(&tree), n);
        let sc = by_id(&tree, "sc").unwrap();
        let mut at = sc;
        let mut copied = 0;
        while let Some(&child) = tree.children(at).first() {
            assert_eq!(tree.children(at).len(), 1);
            assert_eq!(name(&tree, child), "i");
            copied += 1;
            at = child;
        }
        assert_eq!(
            copied,
            2,
            "limit {n}: the selectedcontent is {} deep",
            n - 2
        );
    }
}

/// Fragments: random markup at limits 4, 8 and 12 for six context elements; no node of the
/// fragment is deeper than the limit (the fragment's root holds the nodes the algorithm
/// returns, 0 deep), and the tree is a tree.
#[test]
fn fragments_stay_inside_the_depth_limit() {
    let mut wrong = Vec::new();
    let mut rng = Rng::new(7);
    for case in 0..3_000 {
        let mut input = String::new();
        for _ in 0..2 + rng.below(30) {
            input.push_str(PIECES[rng.below(PIECES.len())]);
        }
        let context = ["div", "template", "table", "tr", "select", "body"][case % 6];
        let n = [4, 8, 12][case % 3];
        let tree = parse_fragment(
            &input,
            &Context {
                namespace: Namespace::Html,
                local_name: context.to_string(),
            },
            &deep(n),
        );
        let found = faults(&tree, &[]);
        let depth = max_element_depth(&tree);
        if depth > n || !found.is_empty() {
            wrong.push(format!(
                "context {context}, limit {n}: {depth} deep, {found:?}: `{input}`"
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} wrong: {:#?}",
        wrong.len(),
        &wrong[..wrong.len().min(5)]
    );
}

/// A small xorshift generator: the same seed gives the same case.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

/// The markup of the random cases.
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
    "</span>",
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
    "<object>",
    "<pre>",
    "<br>",
    "<frameset>",
    "<annotation-xml encoding=text/html>",
    "<b id=1>",
    "<b id=2>",
    "<b id=3>",
    "<u>",
    "<s>",
    "<script>s</script>",
    "<script>s</script>",
    "<script>s</script>",
    "<svg><script>v</script>",
    "<svg><script><foreignObject><script>w</script>",
    "<math><mi><script>m</script>",
    "<template><script>t</script></template>",
    "<table><script>u</script>",
];

/// A write whose answer the random hook does not read.
fn tried<T>(_: Result<T, surf_html::WriteError>) {}

/// A hook that writes at random on the nodes it has seen and made: moves, removals, clones,
/// template contents, fragments, text.
struct Random {
    rng: Rng,
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
        match self.rng.below(12) {
            0 | 1 => tried(tree.append_child(a, b)),
            2 => tried(tree.insert_before(a, b, Some(c))),
            3 | 4 => tried(tree.remove_node(a)),
            5 => tried(tree.replace_child(a, b, c)),
            6 => tried(tree.move_before(a, b, Some(c))),
            7 => {
                if let Ok(new) = tree.create_element_ns(Namespace::Html, "template") {
                    self.made.extend(reach(tree, &[new]));
                    tried(tree.append_child(a, new));
                    if let Some(contents) = contents_of(tree, new) {
                        tried(tree.append_child(contents, b));
                    }
                }
            }
            8 => {
                if let Ok(copy) = tree.clone_node(a, true) {
                    self.made.extend(reach(tree, &[copy]));
                    tried(tree.append_child(b, copy));
                }
            }
            9 => {
                if let Ok(fragment) = tree.create_document_fragment() {
                    self.made.insert(fragment);
                    tried(tree.append_child(fragment, a));
                    tried(tree.append_child(fragment, b));
                    tried(tree.insert_before(c, fragment, None));
                }
            }
            10 => {
                let (first, last) = chain(tree, 1 + self.rng.below(6));
                self.made.extend(reach(tree, &[first]));
                tried(tree.append_child(last, a));
                tried(tree.append_child(b, first));
            }
            _ => tried(tree.set_text_content(a, "t")),
        }
    }
}

impl ScriptHook for Random {
    fn run(&mut self, tree: &mut Tree, script: NodeId) {
        self.calls.push((script, None));
        self.seen.insert(script);
        self.seen.extend(connected(tree));
        let pool: Vec<NodeId> = self.seen.union(&self.made).copied().collect();
        for _ in 0..1 + self.rng.below(5) {
            self.write(tree, &pool);
        }
    }

    fn skip(&mut self, _: &Tree, script: NodeId, reason: NotRun) {
        self.calls.push((script, Some(reason)));
        self.seen.insert(script);
    }
}

/// One random case: what is wrong with its tree and its calls, and the hooks that ran.
fn random_case(seed: u64, n: usize) -> (String, Vec<String>, usize) {
    let mut rng = Rng::new(seed);
    let mut input = String::new();
    for _ in 0..4 + rng.below(50) {
        input.push_str(PIECES[rng.below(PIECES.len())]);
    }
    let mut hook = Random {
        rng,
        calls: Vec::new(),
        seen: BTreeSet::new(),
        made: BTreeSet::new(),
    };
    let tree = parse_document_with_hook(&input, &deep(n), &mut hook);
    let mut wrong = Vec::new();
    let known: Vec<NodeId> = hook.seen.union(&hook.made).copied().collect();
    wrong.extend(faults(&tree, &known));
    let depth = max_element_depth(&tree);
    if depth > n {
        wrong.push(format!("{depth} deep at a limit of {n}"));
    }
    let mut called = HashSet::new();
    for &(node, _) in &hook.calls {
        if !called.insert(node) {
            wrong.push(format!("{node:?} called twice"));
        }
    }
    let mut starts = known;
    starts.push(tree.root());
    for node in reach(&tree, &starts) {
        if is_script_element(&tree, node) && !hook.made.contains(&node) && !called.contains(&node) {
            wrong.push(format!("{node:?} no call"));
        }
    }
    let runs = hook.calls.iter().filter(|c| c.1.is_none()).count();
    (input, wrong, runs)
}

/// Random writes that move nodes between deep and shallow places, in and out of the tree and
/// of template contents, and random markup at limits 6, 8, 10 and 12, 24,000 parses: every
/// parse returns within 5 s, every tree is a tree no deeper than the limit, every script
/// element the parser inserted had one call.
#[test]
fn random_moves_between_depths_keep_the_limit_a_tree_and_one_call() {
    let (send, receive) = mpsc::channel();
    const CASES: u64 = 24_000;
    std::thread::spawn(move || {
        for seed in 0..CASES {
            let n = [6, 8, 10, 12][(seed % 4) as usize];
            let result = std::panic::catch_unwind(|| random_case(seed, n));
            let report = match result {
                Ok((_, wrong, runs)) if wrong.is_empty() => (None, runs),
                Ok((input, wrong, runs)) => (
                    Some(format!("seed {seed}, limit {n}, `{input}`: {wrong:#?}")),
                    runs,
                ),
                Err(_) => (Some(format!("seed {seed} panicked")), 0),
            };
            if send.send(report).is_err() {
                return;
            }
        }
    });
    let mut wrong = Vec::new();
    let mut runs = 0;
    for seed in 0..CASES {
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
        "{} of {CASES}: {:#?}",
        wrong.len(),
        &wrong[..wrong.len().min(5)]
    );
    assert!(runs as u64 >= CASES / 2, "{runs} hooks ran");
}
